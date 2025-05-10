#![allow(deprecated)]

use bevy_egui::egui::Image;
use egui_extras::image::RetainedImage;
use font_awesome_as_a_crate as fa;
use std::sync::LazyLock;

// TODO: Migrate RetainedImage to Image 
//pub static test: LazyLock<Image<'static>> = LazyLock::new(Image::from_bytes("fa_grid", fa::svg(fa::Type::Solid, "table-cells").unwrap().as_bytes()));

pub static FA_GRID: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_grid",
    fa::svg(fa::Type::Solid, "table-cells").unwrap()
).unwrap());

pub static FA_CUBES: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_cubes",
    fa::svg(fa::Type::Solid, "cubes").unwrap()
).unwrap());

pub static FA_REFRESH: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_refresh",
    fa::svg(fa::Type::Solid, "arrows-rotate").unwrap()
).unwrap());

pub static FA_ARROWS_MULTI: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_arrows_multi",
    fa::svg(fa::Type::Solid, "arrows-up-down-left-right").unwrap()
).unwrap());

pub static FA_EYE: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_eye",
    fa::svg(fa::Type::Solid, "eye").unwrap()
).unwrap());

pub static FA_TRASH: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_trash",
    fa::svg(fa::Type::Solid, "trash-can").unwrap()
).unwrap());

pub static FA_EDIT: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_pen_to_square",
    fa::svg(fa::Type::Solid, "pen-to-square").unwrap()
).unwrap());

pub static FA_SEARCH: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_magnifying_glass",
    fa::svg(fa::Type::Solid, "magnifying-glass").unwrap()
).unwrap());

// Pro icon :(
/* pub static FA_GLOBE: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_globe",
    fa::svg(fa::Type::Regular, "globe").unwrap()
).unwrap()); */

pub static FA_CIRCLE: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_circle",
    fa::svg(fa::Type::Regular, "circle").unwrap()
).unwrap());

pub static FA_PLUS: LazyLock<RetainedImage> = LazyLock::new(|| egui_extras::RetainedImage::from_svg_str(
    "fa_plus",
    fa::svg(fa::Type::Solid, "plus").unwrap()
).unwrap());