/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A custom implementation of the "text analysis source" interface so that
//! we can convey data to the `FontFallback::map_characters` method.

#![allow(non_snake_case)]

use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::mem;
use std::ptr::{self, null};
use std::sync::atomic::AtomicUsize;
use winapi::ctypes::wchar_t;
use winapi::shared::basetsd::UINT32;
use winapi::shared::guiddef::REFIID;
use winapi::shared::minwindef::{FALSE, TRUE, ULONG};
use winapi::shared::winerror::{E_INVALIDARG, S_OK};
use winapi::um::dwrite::IDWriteNumberSubstitution;
use winapi::um::dwrite::IDWriteTextAnalysisSource;
use winapi::um::dwrite::IDWriteTextAnalysisSourceVtbl;
use winapi::um::dwrite::DWRITE_NUMBER_SUBSTITUTION_METHOD;
use winapi::um::dwrite::DWRITE_READING_DIRECTION;
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winnt::HRESULT;
use wio::com::ComPtr;

use super::DWriteFactory;
use crate::com_helpers::Com;
use crate::helpers::ToWide;

/// The Rust side of a custom text analysis source implementation.
pub trait TextAnalysisSourceMethods {
    /// Determine the locale for a range of text.
    ///
    /// Return locale and length of text (in utf-16 code units) for which the
    /// locale is valid.
    fn get_locale_name<'a>(&'a self, text_position: u32) -> (Cow<'a, str>, u32);

    /// Get the text direction for the paragraph.
    fn get_paragraph_reading_direction(&self) -> DWRITE_READING_DIRECTION;
}

#[repr(C)]
pub struct CustomTextAnalysisSourceImpl {
    // NB: This must be the first field.
    _refcount: AtomicUsize,
    inner: Box<dyn TextAnalysisSourceMethods>,
    text: Vec<wchar_t>,
    number_subst: NumberSubstitution,
    locale_buf: Vec<wchar_t>,
}

/// A wrapped version of an `IDWriteNumberSubstitution` object.
pub struct NumberSubstitution {
    native: UnsafeCell<ComPtr<IDWriteNumberSubstitution>>,
}

// TODO: implement Clone, for convenience and efficiency?

static TEXT_ANALYSIS_SOURCE_VTBL: IDWriteTextAnalysisSourceVtbl = IDWriteTextAnalysisSourceVtbl {
    parent: implement_iunknown!(static IDWriteTextAnalysisSource, CustomTextAnalysisSourceImpl),
    GetLocaleName: CustomTextAnalysisSourceImpl_GetLocaleName,
    GetNumberSubstitution: CustomTextAnalysisSourceImpl_GetNumberSubstitution,
    GetParagraphReadingDirection: CustomTextAnalysisSourceImpl_GetParagraphReadingDirection,
    GetTextAtPosition: CustomTextAnalysisSourceImpl_GetTextAtPosition,
    GetTextBeforePosition: CustomTextAnalysisSourceImpl_GetTextBeforePosition,
};

impl CustomTextAnalysisSourceImpl {
    /// Create a new custom TextAnalysisSource for the given text and a trait
    /// implementation.
    ///
    /// Note: this method only supports a single `NumberSubstitution` for the
    /// entire string.
    pub fn from_text_and_number_subst_native(
        inner: Box<dyn TextAnalysisSourceMethods>,
        text: Vec<wchar_t>,
        number_subst: NumberSubstitution,
    ) -> ComPtr<IDWriteTextAnalysisSource> {
        assert!(text.len() <= (std::u32::MAX as usize));
        unsafe {
            ComPtr::from_raw(
                CustomTextAnalysisSourceImpl {
                    _refcount: AtomicUsize::new(1),
                    inner,
                    text,
                    number_subst,
                    locale_buf: Vec::new(),
                }
                .into_interface(),
            )
        }
    }
}

impl Com<IDWriteTextAnalysisSource> for CustomTextAnalysisSourceImpl {
    type Vtbl = IDWriteTextAnalysisSourceVtbl;
    #[inline]
    fn vtbl() -> &'static IDWriteTextAnalysisSourceVtbl {
        &TEXT_ANALYSIS_SOURCE_VTBL
    }
}

impl Com<IUnknown> for CustomTextAnalysisSourceImpl {
    type Vtbl = IUnknownVtbl;
    #[inline]
    fn vtbl() -> &'static IUnknownVtbl {
        &TEXT_ANALYSIS_SOURCE_VTBL.parent
    }
}

unsafe extern "system" fn CustomTextAnalysisSourceImpl_GetLocaleName(
    this: *mut IDWriteTextAnalysisSource,
    text_position: UINT32,
    text_length: *mut UINT32,
    locale_name: *mut *const wchar_t,
) -> HRESULT {
    let this = CustomTextAnalysisSourceImpl::from_interface(this);
    let (locale, text_len) = this.inner.get_locale_name(text_position);
    // TODO(performance): reuse buffer (and maybe use smallvec)
    this.locale_buf = locale.as_ref().to_wide_null();
    *text_length = text_len;
    *locale_name = this.locale_buf.as_ptr();
    S_OK
}

unsafe extern "system" fn CustomTextAnalysisSourceImpl_GetNumberSubstitution(
    this: *mut IDWriteTextAnalysisSource,
    text_position: UINT32,
    text_length: *mut UINT32,
    number_substitution: *mut *mut IDWriteNumberSubstitution,
) -> HRESULT {
    let this = CustomTextAnalysisSourceImpl::from_interface(this);
    if text_position >= (this.text.len() as u32) {
        return E_INVALIDARG;
    }
    (*this.number_subst.native.get()).AddRef();
    *text_length = (this.text.len() as UINT32) - text_position;
    *number_substitution = (*this.number_subst.native.get()).as_raw();
    S_OK
}

unsafe extern "system" fn CustomTextAnalysisSourceImpl_GetParagraphReadingDirection(
    this: *mut IDWriteTextAnalysisSource,
) -> DWRITE_READING_DIRECTION {
    let this = CustomTextAnalysisSourceImpl::from_interface(this);
    this.inner.get_paragraph_reading_direction()
}

unsafe extern "system" fn CustomTextAnalysisSourceImpl_GetTextAtPosition(
    this: *mut IDWriteTextAnalysisSource,
    text_position: UINT32,
    text_string: *mut *const wchar_t,
    text_length: *mut UINT32,
) -> HRESULT {
    let this = CustomTextAnalysisSourceImpl::from_interface(this);
    if text_position >= (this.text.len() as u32) {
        *text_string = null();
        *text_length = 0;
        return S_OK;
    }
    *text_string = this.text.as_ptr().add(text_position as usize);
    *text_length = (this.text.len() as UINT32) - text_position;
    S_OK
}

unsafe extern "system" fn CustomTextAnalysisSourceImpl_GetTextBeforePosition(
    this: *mut IDWriteTextAnalysisSource,
    text_position: UINT32,
    text_string: *mut *const wchar_t,
    text_length: *mut UINT32,
) -> HRESULT {
    let this = CustomTextAnalysisSourceImpl::from_interface(this);
    if text_position == 0 || text_position > (this.text.len() as u32) {
        *text_string = null();
        *text_length = 0;
        return S_OK;
    }
    *text_string = this.text.as_ptr();
    *text_length = text_position;
    S_OK
}

impl NumberSubstitution {
    pub fn new(
        subst_method: DWRITE_NUMBER_SUBSTITUTION_METHOD,
        locale: &str,
        ignore_user_overrides: bool,
    ) -> NumberSubstitution {
        unsafe {
            let mut native: *mut IDWriteNumberSubstitution = ptr::null_mut();
            let hr = (*DWriteFactory()).CreateNumberSubstitution(
                subst_method,
                locale.to_wide_null().as_ptr(),
                if ignore_user_overrides { TRUE } else { FALSE },
                &mut native,
            );
            assert_eq!(hr, 0, "error creating number substitution");
            NumberSubstitution {
                native: UnsafeCell::new(ComPtr::from_raw(native)),
            }
        }
    }
}
