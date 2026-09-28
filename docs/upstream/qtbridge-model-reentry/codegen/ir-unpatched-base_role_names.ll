define i64 @_RNvMs1_NtNtCsdlgj6ivNBuA_19qtbridge_interfaces11qlist_model10proxy_rustINtNtB9_16genericrustproxy16GenericRustProxyNtNtNtB7_16proxy_cpp_bridge3ffi18QListModelProxyCppDNtB5_17QListModelAdapterEL_E15base_role_names(ptr noundef nonnull align 8 captures(none) %self, ptr noundef nonnull %reference.0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(112) %reference.1) unnamed_addr #1 personality ptr @rust_eh_personality {
start:
  %e.i = alloca [1 x i8], align 1
  %guard.sroa.11.i = alloca [16 x i8], align 8
  %_6 = alloca [16 x i8], align 8
  %0 = getelementptr inbounds nuw i8, ptr %self, i64 56
  %_5 = load ptr, ptr %0, align 8, !noundef !4
  %1 = icmp eq ptr %_5, null
  br i1 %1, label %bb3, label %bb4, !prof !7

bb3:                                              ; preds = %start
; call core::option::expect_failed
  tail call void @_RNvNtCs8Mbv00yxnRz_4core6option13expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_1f2d3d9fa2eb4e863c17c3e79dc6c8a3, i64 noundef 18, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_0330a62193fa312878e32e5f1e0ae840) #20
  unreachable

bb4:                                              ; preds = %start
  call void @llvm.lifetime.start.p0(ptr nonnull %_6)
  call void @llvm.lifetime.start.p0(ptr nonnull %guard.sroa.11.i)
  %_17.0.i = load ptr, ptr %self, align 8, !noalias !172, !nonnull !4, !noundef !4
  %2 = getelementptr inbounds nuw i8, ptr %self, i64 8
  %_17.1.i = load ptr, ptr %2, align 8, !noalias !172, !nonnull !4, !align !8, !noundef !4
  %3 = getelementptr inbounds nuw i8, ptr %_17.1.i, i64 16
  %4 = load i64, ptr %3, align 8, !range !6, !invariant.load !4, !noalias !172
  %5 = add nsw i64 %4, -1
  %6 = tail call i64 @llvm.umax.i64(i64 %4, i64 8)
  %7 = add nsw i64 %6, -1
  %8 = and i64 %7, -16
  %9 = getelementptr inbounds nuw i8, ptr %_17.0.i, i64 %8
  %10 = and i64 %5, -8
  %11 = getelementptr inbounds nuw i8, ptr %9, i64 24
  %_18.0.i = getelementptr inbounds nuw i8, ptr %11, i64 %10
  %_4.i = icmp eq ptr %_18.0.i, %reference.0
  br i1 %_4.i, label %bb1.i, label %bb2.i, !prof !72

bb2.i:                                            ; preds = %bb4
; call core::panicking::panic_fmt
  tail call void @_RNvNtCs8Mbv00yxnRz_4core9panicking9panic_fmt(ptr noundef nonnull @alloc_2905f756ccd7229346596936eae9c99c, ptr noundef nonnull inttoptr (i64 161 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_a1b166fd47004c154b22d3743f6278c3) #19, !noalias !172
  unreachable

bb1.i:                                            ; preds = %bb4
  %_8.i = getelementptr inbounds nuw i8, ptr %self, i64 16
  %_19.sroa.0.0.copyload.i = load i64, ptr %_8.i, align 8, !noalias !172
  %_19.sroa.4.0._8.sroa_idx.i = getelementptr inbounds nuw i8, ptr %self, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, ptr noundef nonnull align 8 dereferenceable(16) %_19.sroa.4.0._8.sroa_idx.i, i64 16, i1 false), !noalias !172
  store i64 1, ptr %_8.i, align 8, !noalias !172
  store ptr %reference.0, ptr %_19.sroa.4.0._8.sroa_idx.i, align 8, !noalias !172
  %_20.sroa.5.0._23.sroa_idx.i = getelementptr inbounds nuw i8, ptr %self, i64 32
  store ptr %reference.1, ptr %_20.sroa.5.0._23.sroa_idx.i, align 8, !noalias !172
  %12 = icmp eq i64 %_19.sroa.0.0.copyload.i, 2
  br i1 %12, label %bb2.i2, label %_RNvMNtCs8Mbv00yxnRz_4core6resultINtB2_6ResultINtNtNtCs5KJ1RkK0Jft_10cxx_qt_lib4core5qhash5QHashNtNtBK_20qhash_i32_qbytearray24QHashPair_i32_QByteArrayENtNtNtCsdlgj6ivNBuA_19qtbridge_interfaces13object_access18rust_object_access18RustObjAccessErrorE6expectB2v_.exit

bb2.i2:                                           ; preds = %bb1.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_19.sroa.4.0._8.sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, i64 16, i1 false), !noalias !172
  store i64 2, ptr %_8.i, align 8, !noalias !172
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.sroa.11.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %e.i), !noalias !176
  store i8 2, ptr %e.i, align 1, !noalias !176
; call core::result::unwrap_failed
  call void @_RNvNtCs8Mbv00yxnRz_4core6result13unwrap_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_34f45f94e90b830ac5952ae743778899, i64 noundef 36, ptr noundef nonnull %e.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @vtable.0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_0330a62193fa312878e32e5f1e0ae840) #19, !noalias !181
  unreachable

_RNvMNtCs8Mbv00yxnRz_4core6resultINtB2_6ResultINtNtNtCs5KJ1RkK0Jft_10cxx_qt_lib4core5qhash5QHashNtNtBK_20qhash_i32_qbytearray24QHashPair_i32_QByteArrayENtNtNtCsdlgj6ivNBuA_19qtbridge_interfaces13object_access18rust_object_access18RustObjAccessErrorE6expectB2v_.exit: ; preds = %bb1.i
  %13 = getelementptr inbounds nuw i8, ptr %_6, i64 8
  call void @"rust$bridge$cxxbridge1$198$QListModelProxyCpp$base_role_names"(ptr noundef nonnull %_5, ptr noundef nonnull %13) #15
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_19.sroa.4.0._8.sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, i64 16, i1 false), !noalias !172
  store i64 %_19.sroa.0.0.copyload.i, ptr %_8.i, align 8, !noalias !172
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.sroa.11.i)
  call void @llvm.experimental.noalias.scope.decl(metadata !181)
  %t.sroa.0.0.copyload.i = load i64, ptr %13, align 8, !alias.scope !181, !noalias !182
  call void @llvm.lifetime.end.p0(ptr nonnull %_6)
  ret i64 %t.sroa.0.0.copyload.i
}
