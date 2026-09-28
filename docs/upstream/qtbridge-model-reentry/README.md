# Upstream report: model-proxy exclusive receiver violates synchronous Qt re-entry

Target: `qt/qtbridge-rust`, Qt Bridges (`QTBRIDGES`) issue tracker.
Classification: **qtbridge model-proxy aliasing/soundness defect**.

## Summary

A mutable QListModel slot calling `reset()` aborts in optimised builds when a
QML view synchronously asks for `roleNames()` during `beginResetModel()`.
qtbridge holds an exclusive Rust reference to its model proxy across the
foreign call, which re-enters that same proxy through the C++ back-pointer.
The resulting aliasing contract permits elimination of the handoff stores
needed by the callback. Debug passes; release fails. A qtbridge-only change
to shared outbound Rust proxy receivers fixes the tested optimised reset.

This report needs no RowPlay source. The attached reproducer, patch and IR
are preserved from the completed macOS isolation work of 2026-09-29; the
experiments were not rerun while preparing this report.

## Versions and platforms

| Component | Version |
|---|---|
| qtbridge / qtbridge-interfaces | 0.3.0 |
| CXX / cxx-build | 1.0.198 |
| CXX-Qt / cxx-qt-lib | 0.10.0 |
| cxx-gen | 0.7.198 |
| Rust | 1.98.1 (LLVM 22.1.8) |
| Qt | 6.11.2 |

The original failure reproduces on Linux x86_64 and macOS arm64. The
controlled matrix below was completed on macOS 27.0 (26A428), arm64,
Apple clang 21.0.0. It includes native Cocoa reproduction and an independent
Cocoa/Metal RowPlay release gate abort at step 5.

## Reproducer and results

[Source and locked dependency graph](reproducer/) provide three correctly
wired exposure paths, selected one at a time with Cargo features:

| Variant | Debug | Optimised release |
|---|---|---|
| `singleton` | PASS 3/3 | FAIL 3/3 |
| `creatable` | PASS 3/3 | FAIL 3/3 |
| `initial` (`set_initial_object` + required root property) | PASS 3/3 | FAIL 3/3 |

All failures contain `Failed to borrow for role_names: BorrowError` and
abort at the CXX FFI boundary. Success means the reset returned, the view's
count changed from 0 to 1, and `reset survived` was printed before exit 0.

Singleton registration is not required. The preserved `initial_selfref`
variant is an explicitly invalid historical control: it has no required
root property, reports an initial-property error, and never executes the
reset. Its earlier apparent pass was not evidence.

| Controlled singleton build | Result |
|---|---|
| Ordinary release | FAIL 3/3 |
| Only qtbridge-interfaces at O0 | PASS 3/3 |
| Patched qtbridge, debug | PASS 3/3 |
| Patched qtbridge, release | PASS 5/5 |
| Patched qtbridge, release, `codegen-units=1` | PASS 3/3 |

The patched three-variant matrix also passes 3/3 per variant per profile.
Only qtbridge source changed: Qt, Rust, CXX, CXX-Qt, cxx-qt-lib, cxx-gen,
reproducer and QML were held constant. O0 is containment, not a sound
source-level repair.

To use this bundle independently, copy this directory outside RowPlay and
place an upstream checkout beside `reproducer/`:

```sh
git clone https://github.com/qt/qtbridge-rust.git qtbridge
git -C qtbridge checkout d9a89bc
cd reproducer
export QMAKE="$HOME/Qt/6.11.2/macos/bin/qmake"
export DYLD_FRAMEWORK_PATH="$HOME/Qt/6.11.2/macos/lib"
export QT_QPA_PLATFORM=offscreen
cargo +1.98.1 run --locked --features singleton
cargo +1.98.1 run --locked --release --features singleton
cargo +1.98.1 run --locked --release --features singleton \
  --config profile.release.package.qtbridge-interfaces.opt-level=0
cargo +1.98.1 run --locked --release --features singleton \
  --config profile.release.codegen-units=1
```

Use `creatable` or `initial` instead of `singleton` for the other valid
variants. On Linux, select the installed Qt's qmake and omit
`DYLD_FRAMEWORK_PATH`; use the local QPA or Xvfb as appropriate. The scratch
macOS binary has no LC_RPATH, hence the explicit framework path. The
lockfile aligns cxx-gen's patch with CXX; do not regenerate it casually.
The relative path in Cargo.toml and its standalone `[workspace]` boundary
are report-packaging changes only; model and QML sources are preserved.

Apply the attachment from the bundle root, then run the same commands with
the O0 override absent:

```sh
git -C qtbridge apply ../qtbridge-0.3.0-shared-receiver.patch
```

## Callback chain and soundness issue

```text
mutable Rust model operation (slot holds RefMut)
→ qtbridge outbound base model notification (&mut Rust proxy)
→ Qt beginResetModel
→ synchronous modelAboutToBeReset
→ QML delegate model asks roleNames
→ C++ proxy re-enters the same Rust proxy through m_rustProxy
```

`try_store_handle_and_call_cpp_mut` is intended to store
`BorrowState::Mutable` (tag plus model fat pointer), call C++, then restore
the old state. With that helper inlined into the exclusive proxy receiver,
the receiver is `noalias`. The foreign call receives a pointer loaded from
the proxy; its independent back-pointer access contradicts the live Rust
exclusive reference. Under the supplied contract, the handoff writes can
be removed before the old state is restored.

The callback consequently sees no usable handoff, falls back to
`RefCell::try_borrow`, conflicts with the slot's mutable borrow, and panics.
CXX's abort when that panic reaches FFI is expected safety behaviour.
LLVM is optimising according to the contract supplied by qtbridge; this
is not an LLVM bug or miscompilation.

The critical bridge is handwritten `qtbridge-interfaces` with direct
`#[cxx::bridge]`. CXX transports its declared contract. CXX-Qt does not
generate this proxy contract, and no separate CXX defect is demonstrated.

## Source-level correction and codegen

The [tested patch](qtbridge-0.3.0-shared-receiver.patch) changes only
`crates/qtbridge-interfaces/src/qlist_model/proxy_rust.rs`: outbound wrappers
take `&self`, and callers use `&*proxy`. The model argument remains
`&mut dyn QListModelAdapter`. The actual C++ Qt model operation still uses
the appropriate pinned mutable C++ proxy. The patch changes 25 lines;
the reset receiver/caller changes are the essential example:

```diff
- unsafe { &mut *proxy }.base_begin_reset_model(&mut *self);
+ unsafe { &*proxy }.base_begin_reset_model(&mut *self);
- pub fn base_begin_reset_model(&mut self, mut_ref: &mut dyn QListModelAdapter) {
+ pub fn base_begin_reset_model(&self, mut_ref: &mut dyn QListModelAdapter) {
      call_cpp_impl!(mut self, mut_ref, base_begin_reset_model())
  }
```

The same receiver pattern exists in QListModel, QAbstractItemModel and
QTableModel and should be reviewed upstream. Runtime verification here is
limited to QListModel reset; it does not claim coverage of all notifications
or of the other two model implementations.

[Optimised IR and final-binary assembly excerpts](codegen/) show:

| Observation | Unpatched | Shared receiver patch |
|---|---|---|
| Rust proxy receiver | `noalias` | no exclusive `noalias` |
| Before C++ reset call | no Mutable tag/fat-pointer handoff stores | tag `2`, model pointer and vtable stored |
| Re-entry | fallback borrow and abort | intended handoff consumed |

The existing shared `base_role_names` path preserves its handoff and is an
in-crate control. IR emission used one codegen unit; the ordinary release
binaries used 16. Final-binary disassembly agrees for 0.3.0. This completed
comparison supersedes the initial isolated IR inspection.

## Independent controls and version history

A pure CXX-Qt 0.10.0 QAbstractListModel following its documented custom-base
pattern passes debug 3/3 and release 3/3 for both creatable and
`qml_singleton` variants. The reset synchronously re-enters `roleNames`
and successfully reads Rust state. Its generated representation does not
establish the problematic qtbridge proxy exclusive-reference contract.
CXX-Qt is not implicated by this evidence.

qtbridge 0.2 contains the same latent receiver/handoff defect. Ordinary
release with 16 codegen units passes 3/3 because the handoff helper remains
out of line; release with `codegen-units=1` fails 3/3 as it inlines into the
exclusive proxy frame. Version 0.2 was not sounder. This makes
`codegen-units=1` a useful local/upstream regression probe alongside the
ordinary release case.

Queued/deferred reset was tested in RowPlay and fails at the same fallback
borrow. RowPlay PR #144 keeps only the per-package O0 mitigation and adds
a release quick gate. RowPlay issue #143 remains open until a fixed upstream
release is adopted and the gates pass with the O0 override removed.

## Duplicate search and filing status (2026-09-29)

No matching report was found in the accessible Qt Jira and Gerrit searches
for qtbridge/reset, aliasing, role_names and BorrowError. Related records:

- [QTBRIDGES-332](https://bugreports.qt.io/browse/QTBRIDGES-332): handling
  panics from dropped/already-borrowed objects, rather than this optimised
  outbound model-proxy handoff defect.
- [QTBRIDGES-183](https://bugreports.qt.io/browse/QTBRIDGES-183): an earlier
  Miri finding in object-access borrow handles.
- [Gerrit 758088](https://codereview.qt-project.org/c/qt/qtbridge-rust/+/758088):
  merged mutable-dispatch receiver correction; these model wrappers remain
  a separate path. Its abandoned duplicate is 758089.

The GitHub mirror has issues and pull requests disabled. The Qt Jira session
is logged out. This report is prepared for filing; no upstream issue/change
has been created. Replace this status with the upstream link after filing.

## Artifact provenance

Sources: completed macOS isolation session in
`/private/tmp/rowplay143/session2/`, with qtbridge 0.3.0 commit `d9a89bc`.
The isolation compared the upstream crate sources with crates.io 0.3.0.
`reproducer/` preserves the matrix Rust/QML and lockfile; only the manifest
path and standalone workspace boundary were adapted for portability.
The patch is the existing tested upstream diff, not a production dependency.
Codegen files are copied excerpts from that session, not regenerated here.
The upstream patch retains the original Qt source licensing terms; the
standalone reproduction sources are GPL-3.0-or-later.
