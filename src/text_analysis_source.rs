/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::UnsafeCell;

use winapi::um::dwrite::IDWriteTextAnalysisSource;

use super::*;

pub struct TextAnalysisSource {
    native: UnsafeCell<ComPtr<IDWriteTextAnalysisSource>>,
}

impl TextAnalysisSource {
    pub unsafe fn as_ptr(&self) -> *mut IDWriteTextAnalysisSource {
        (*self.native.get()).as_ptr()
    }
}
