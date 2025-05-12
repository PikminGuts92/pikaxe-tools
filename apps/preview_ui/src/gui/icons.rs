use bevy_egui::egui::{Image, Vec2};
use font_awesome_as_a_crate as fa;
use std::sync::LazyLock;

const ICON_SIZE: Vec2 = Vec2::splat(12.);

pub static FA_GRID: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_grid",
    fa::svg(fa::Type::Solid, "table-cells").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_CUBES: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_cubes",
    fa::svg(fa::Type::Solid, "cubes").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_REFRESH: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_refresh",
    fa::svg(fa::Type::Solid, "arrows-rotate").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_ARROWS_MULTI: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_arrows_multi",
    fa::svg(fa::Type::Solid, "arrows-up-down-left-right").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_EYE: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_eye",
    fa::svg(fa::Type::Solid, "eye").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_TRASH: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_trash",
    fa::svg(fa::Type::Solid, "trash-can").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_EDIT: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_pen_to_square",
    fa::svg(fa::Type::Solid, "pen-to-square").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_SEARCH: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_magnifying_glass",
    fa::svg(fa::Type::Solid, "magnifying-glass").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

// Pro icon :(
/* pub static FA_GLOBE: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_globe",
    fa::svg(fa::Type::Regular, "globe").unwrap().as_bytes()
)); */

pub static FA_CIRCLE: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_circle",
    fa::svg(fa::Type::Regular, "circle").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));

pub static FA_PLUS: LazyLock<Image> = LazyLock::new(|| Image::from_bytes(
    "fa_plus",
    fa::svg(fa::Type::Solid, "plus").unwrap().as_bytes()
).fit_to_exact_size(ICON_SIZE));