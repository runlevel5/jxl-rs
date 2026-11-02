// Copyright (c) the JPEG XL Project Authors. All rights reserved.
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#![allow(unsafe_code)]
#![allow(clippy::identity_op)]

// The backend requires AltiVec at compile time, which every powerpc64 target enables. The ISA
// level is a compile-time choice as well (`-C target-cpu=pwr8`, `pwr9`, ...; powerpc64le
// implies POWER8): LLVM does not inline the AltiVec intrinsics into functions that enable
// features through `#[target_feature]`, so detecting a level at runtime would leave every
// vector operation as a function call, see `vsx.rs`.
#[cfg(all(feature = "vmx", target_feature = "altivec"))]
pub(super) mod vsx;

#[macro_export]
macro_rules! simd_function {
    (
        $dname:ident,
        $descr:ident: $descr_ty:ident,
        $(#[$($attr:meta)*])*
        $pub:vis fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty )? $body: block
    ) => {
        #[inline(always)]
        $(#[$($attr)*])*
        $pub fn $name<$descr_ty: $crate::SimdDescriptor>($descr: $descr_ty, $($arg: $ty),*) $(-> $ret)? $body
        $(#[$($attr)*])*
        $pub fn $dname($($arg: $ty),*) $(-> $ret)? {
            #[allow(unused)]
            use $crate::SimdDescriptor;
            $crate::simd_function_body_vsx!($name($($arg: $ty),*) $(-> $ret)?; ($($arg),*));
            $name($crate::ScalarDescriptor::new().unwrap(), $($arg),*)
        }
    };
}

#[cfg(all(feature = "vmx", target_feature = "altivec"))]
#[doc(hidden)]
#[macro_export]
macro_rules! simd_function_body_vsx {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty )?; ($($val:expr),* $(,)?)) => {
        if let Some(d) = $crate::VsxDescriptor::<$crate::Power9>::new() {
            return d.call(
                #[inline(always)]
                |d| $name(d, $($val),*),
            );
        }
        if let Some(d) = $crate::VsxDescriptor::<$crate::Power8>::new() {
            return $name(d, $($val),*);
        }
        if let Some(d) = $crate::VsxDescriptor::<$crate::Vmx>::new() {
            return $name(d, $($val),*);
        }
    };
}

#[cfg(not(all(feature = "vmx", target_feature = "altivec")))]
#[doc(hidden)]
#[macro_export]
macro_rules! simd_function_body_vsx {
    ($($ignore:tt)*) => {};
}

#[macro_export]
macro_rules! test_all_instruction_sets {
    (
        $name:ident
    ) => {
        paste::paste! {
            #[test]
            fn [<$name _scalar>]() {
                use $crate::SimdDescriptor;
                $name($crate::ScalarDescriptor::new().unwrap())
            }
        }

        $crate::test_vsx!($name);
    };
}

#[cfg(all(feature = "vmx", target_feature = "altivec"))]
#[doc(hidden)]
#[macro_export]
macro_rules! test_vsx {
    ($name:ident) => {
        paste::paste! {
            #[test]
            fn [<$name _vmx>]() {
                use $crate::SimdDescriptor;
                let Some(d) = $crate::VsxDescriptor::<$crate::Vmx>::new() else { return; };
                d.call($name)
            }

            #[test]
            fn [<$name _power8>]() {
                use $crate::SimdDescriptor;
                let Some(d) = $crate::VsxDescriptor::<$crate::Power8>::new() else { return; };
                d.call($name)
            }

            #[test]
            fn [<$name _power9>]() {
                use $crate::SimdDescriptor;
                let Some(d) = $crate::VsxDescriptor::<$crate::Power9>::new() else { return; };
                d.call($name)
            }

        }
    };
}

#[cfg(not(all(feature = "vmx", target_feature = "altivec")))]
#[doc(hidden)]
#[macro_export]
macro_rules! test_vsx {
    ($name:ident) => {};
}

#[macro_export]
macro_rules! bench_all_instruction_sets {
    (
        $name:ident,
        $criterion:ident
    ) => {
        use $crate::SimdDescriptor;
        // `simd_function_body_*` does early return; wrap it with an immediately-invoked closure
        (|| {
            $crate::simd_function_body_vsx!(
                $name($criterion: &mut ::criterion::BenchmarkGroup<'_, impl ::criterion::measurement::Measurement>);
                ($criterion, "vsx")
            );
        })();
        $name(
            $crate::ScalarDescriptor::new().unwrap(),
            $criterion,
            "scalar",
        );
    };
}
