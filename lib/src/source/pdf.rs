use std::{collections::VecDeque, path::Path};

use crate::Result;
use serde::{Deserialize, Serialize};
use mixlayer_data::{Frame, JsonMxlData};
use mixlayer_graph::{MxlNode, MxlNodeCtx, MxlSource};
use mixlayer_runtime_ffi::{
    prost::Message,
    protos::{ReadPdfPagesPageText, ReadPdfPagesTextRequest, ReadPdfPagesTextResponse},
    ByteBuffer,
};

extern "C" {
    fn _read_pdf_pages_text(request: *const ByteBuffer) -> *mut ByteBuffer;
}

pub fn read_pdf_pages_text(
    req: mixlayer_runtime_ffi::protos::ReadPdfPagesTextRequest,
) -> Result<ReadPdfPagesTextResponse> {
    let request_bytes: ByteBuffer = req.encode_to_vec().into();
    let response_bytes: Box<ByteBuffer> =
        unsafe { Box::from_raw(_read_pdf_pages_text(&request_bytes)) };

    let response_bytes = response_bytes.into_bytes();

    Ok(mixlayer_runtime_ffi::protos::ReadPdfPagesTextResponse::decode(response_bytes)?)
}

/// Reads lines from a file on the local filesystem
pub struct PdfPageTextSource {
    pdf_path: String,
    pages: VecDeque<ReadPdfPagesPageText>,
}

impl PdfPageTextSource {
    //TODO currentl eagerly reading in all page data but should be streaming
    pub fn new(pdf_path: impl AsRef<Path>) -> Result<Self> {
        let pdf_path = pdf_path.as_ref().to_owned();
        let pdf_path = pdf_path.to_string_lossy().to_string();
        let resp = read_pdf_pages_text(ReadPdfPagesTextRequest {
            file: pdf_path.clone(),
        })?;

        let mut pages = VecDeque::new();
        pages.extend(resp.pages.into_iter());

        Ok(Self { pdf_path, pages })
    }
}

impl MxlSource for PdfPageTextSource {
    type Output = PdfPageText;
}

impl MxlNode for PdfPageTextSource {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        if let Some(page) = self.pages.pop_front() {
            self.send(
                ctx,
                Frame::Data(PdfPageText {
                    file: self.pdf_path.clone(),
                    page: page.page_number,
                    text: page.text,
                }),
            )?;
        } else {
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some(format!("{}", self.pdf_path))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPageText {
    pub file: String,
    pub page: u32,
    pub text: String,
}

impl JsonMxlData for PdfPageText {}
