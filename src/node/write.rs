use std::{path::Path, sync::Arc};

use crate::{error::Result, slot_data::SlotData};

pub(crate) fn process(slot_datas: &[Arc<SlotData>], path: &Path) -> Result<Vec<Arc<SlotData>>> {
    if let Some(slot_data) = slot_datas.get(0) {
        let (width, height) = (slot_data.size.width, slot_data.size.height);

        image::save_buffer(
            &path,
            &image::RgbaImage::from_vec(width, height, slot_data.image.get().to_u8()).unwrap(),
            width,
            height,
            image::ColorType::RGBA(8),
        )
        .unwrap();
    }

    Ok(Vec::new())
}
