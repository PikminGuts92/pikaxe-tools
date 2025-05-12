use bevy_egui::egui::{ColorImage, ImageData, Vec2};
use font_awesome_as_a_crate as fa;
use std::sync::LazyLock;

const ICON_SIZE: Vec2 = Vec2::splat(12.);

pub fn load_icon_image(fa_type: fa::Type, fa_name: &str) -> ImageData {
    egui_extras::image::load_svg_bytes_with_size(fa::svg(fa_type, fa_name).unwrap().as_bytes(), Some(ICON_SIZE.into())).unwrap().into()
}

pub static FA_GRID: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "table-cells"));
pub static FA_CUBES: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "cubes"));
pub static FA_REFRESH: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "arrows-rotate"));
pub static FA_ARROWS_MULTI: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "arrows-up-down-left-right"));
pub static FA_EYE: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "eye"));
pub static FA_TRASH: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "trash-can"));
pub static FA_EDIT: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "pen-to-square"));
pub static FA_SEARCH: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "magnifying-glass"));
//pub static FA_GLOBE: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Regular, "globe")); // Pro icon :(
pub static FA_CIRCLE: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Regular, "circle"));
pub static FA_PLUS: LazyLock<ImageData> = LazyLock::new(|| load_icon_image(fa::Type::Solid, "plus"));