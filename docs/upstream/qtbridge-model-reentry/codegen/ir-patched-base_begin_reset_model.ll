define void @_RNvMs1_NtNtCse01LVDpoV3P_19qtbridge_interfaces11qlist_model10proxy_rustINtNtB9_16genericrustproxy16GenericRustProxyNtNtNtB7_16proxy_cpp_bridge3ffi18QListModelProxyCppDNtB5_17QListModelAdapterEL_E22base_begin_reset_model(ptr noundef nonnull align 8 captures(none) %self, ptr noundef nonnull %mut_ref.0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(112) %mut_ref.1) unnamed_addr #1 personality ptr @rust_eh_personality {
start:
  %e.i = alloca [1 x i8], align 1
  %guard.sroa.11.i = alloca [16 x i8], align 8
  %0 = getelementptr inbounds nuw i8, ptr %self, i64 56
  %_5 = load ptr, ptr %0, align 8, !noundef !4
  %1 = icmp eq ptr %_5, null
  br i1 %1, label %bb3, label %bb4, !prof !7

bb3:                                              ; preds = %start
; call core::option::expect_failed
  tail call void @_RNvNtCs8Mbv00yxnRz_4core6option13expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_1f2d3d9fa2eb4e863c17c3e79dc6c8a3, i64 noundef 18, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_83933a4a7554da19353a185c558252b7) #20
  unreachable

bb4:                                              ; preds = %start
  call void @llvm.lifetime.start.p0(ptr nonnull %guard.sroa.11.i)
  %_18.0.i = load ptr, ptr %self, align 8, !noalias !266, !nonnull !4, !noundef !4
  %2 = getelementptr inbounds nuw i8, ptr %self, i64 8
  %_18.1.i = load ptr, ptr %2, align 8, !noalias !266, !nonnull !4, !align !8, !noundef !4
  %3 = getelementptr inbounds nuw i8, ptr %_18.1.i, i64 16
  %4 = load i64, ptr %3, align 8, !range !6, !invariant.load !4, !noalias !266
  %5 = add nsw i64 %4, -1
  %6 = tail call i64 @llvm.umax.i64(i64 %4, i64 8)
  %7 = add nsw i64 %6, -1
  %8 = and i64 %7, -16
  %9 = getelementptr inbounds nuw i8, ptr %_18.0.i, i64 %8
  %10 = and i64 %5, -8
  %11 = getelementptr inbounds nuw i8, ptr %9, i64 24
  %_19.0.i = getelementptr inbounds nuw i8, ptr %11, i64 %10
  %_4.i = icmp eq ptr %_19.0.i, %mut_ref.0
  br i1 %_4.i, label %bb1.i, label %bb2.i, !prof !72

bb2.i:                                            ; preds = %bb4
; call core::panicking::panic_fmt
  tail call void @_RNvNtCs8Mbv00yxnRz_4core9panicking9panic_fmt(ptr noundef nonnull @alloc_2905f756ccd7229346596936eae9c99c, ptr noundef nonnull inttoptr (i64 161 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_53f9717ee6fdf55bdf5403f12b0de44b) #19, !noalias !266
  unreachable

bb1.i:                                            ; preds = %bb4
  %_8.i = getelementptr inbounds nuw i8, ptr %self, i64 16
  %_20.sroa.0.0.copyload.i = load i64, ptr %_8.i, align 8, !noalias !266
  %_20.sroa.4.0._8.sroa_idx.i = getelementptr inbounds nuw i8, ptr %self, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, ptr noundef nonnull align 8 dereferenceable(16) %_20.sroa.4.0._8.sroa_idx.i, i64 16, i1 false), !noalias !266
  store i64 2, ptr %_8.i, align 8, !noalias !266
  store ptr %mut_ref.0, ptr %_20.sroa.4.0._8.sroa_idx.i, align 8, !noalias !266
  %_21.sroa.5.0._24.sroa_idx.i = getelementptr inbounds nuw i8, ptr %self, i64 32
  store ptr %mut_ref.1, ptr %_21.sroa.5.0._24.sroa_idx.i, align 8, !noalias !266
  %12 = add nsw i64 %_20.sroa.0.0.copyload.i, -1
  %or.cond.i = icmp ult i64 %12, 2
  br i1 %or.cond.i, label %bb2.i1, label %_RNvMNtCs8Mbv00yxnRz_4core6resultINtB2_6ResultuNtNtNtCse01LVDpoV3P_19qtbridge_interfaces13object_access18rust_object_access18RustObjAccessErrorE6expectBO_.exit

bb2.i1:                                           ; preds = %bb1.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_20.sroa.4.0._8.sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, i64 16, i1 false), !noalias !266
  store i64 %_20.sroa.0.0.copyload.i, ptr %_8.i, align 8, !noalias !266
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.sroa.11.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %e.i), !noalias !269
  store i8 2, ptr %e.i, align 1, !noalias !269
; call core::result::unwrap_failed
  call void @_RNvNtCs8Mbv00yxnRz_4core6result13unwrap_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_ffcedded1299125f4f245fdf85216048, i64 noundef 51, ptr noundef nonnull %e.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @vtable.0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_83933a4a7554da19353a185c558252b7) #19
  unreachable

_RNvMNtCs8Mbv00yxnRz_4core6resultINtB2_6ResultuNtNtNtCse01LVDpoV3P_19qtbridge_interfaces13object_access18rust_object_access18RustObjAccessErrorE6expectBO_.exit: ; preds = %bb1.i
  tail call void @"rust$bridge$cxxbridge1$198$QListModelProxyCpp$base_begin_reset_model"(ptr noundef nonnull %_5) #15
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_20.sroa.4.0._8.sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(16) %guard.sroa.11.i, i64 16, i1 false), !noalias !266
  store i64 %_20.sroa.0.0.copyload.i, ptr %_8.i, align 8, !noalias !266
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.sroa.11.i)
  ret void
}
