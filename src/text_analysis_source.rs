/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::UnsafeCell;
use winapi::ctypes::wchar_t;
use winapi::um::dwrite::IDWriteTextAnalysisSource;
use wio::com::ComPtr;

use super::*;

pub struct TextAnalysisSource {
    native: UnsafeCell<ComPtr<IDWriteTextAnalysisSource>>,
}

impl TextAnalysisSource {
    /// Create a new custom TextAnalysisSource for the given text and a trait
    /// implementation.
    ///
    /// Note: this method has no NumberSubsitution specified. See
    /// `from_text_and_number_subst` if you need number substitution.
    pub fn from_text(
        inner: Box<dyn TextAnalysisSourceMethods>,
        text: Vec<wchar_t>,
    ) -> TextAnalysisSource {
        let native = CustomTextAnalysisSourceImpl::from_text_native(inner, text);
        TextAnalysisSource::take(native)
    }

    /// Create a new custom TextAnalysisSource for the given text and a trait
    /// implementation.
    ///
    /// Note: this method only supports a single `NumberSubstitution` for the
    /// entire string.
    pub fn from_text_and_number_subst(
        inner: Box<dyn TextAnalysisSourceMethods>,
        text: Vec<wchar_t>,
        number_subst: NumberSubstitution,
    ) -> TextAnalysisSource {
        let native = CustomTextAnalysisSourceImpl::from_text_and_number_subst_native(
            inner,
            text,
            number_subst,
        );
        TextAnalysisSource::take(native)
    }

    pub unsafe fn as_ptr(&self) -> *mut IDWriteTextAnalysisSource {
        (*self.native.get()).as_raw()
    }

    // TODO: following crate conventions, but there's a safety problem
    pub fn take(native: ComPtr<IDWriteTextAnalysisSource>) -> TextAnalysisSource {
        TextAnalysisSource {
            native: UnsafeCell::new(native),
        }
    }
}
