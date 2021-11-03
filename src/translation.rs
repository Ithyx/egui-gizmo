use egui::{Color32, PointerButton, Response, Stroke};
use glam::{Mat4, Vec3};

use crate::math::{
    intersect_plane, ray_to_plane_origin, ray_to_ray, round_to_interval, segment_to_segment,
};
use crate::painter::Painter3d;
use crate::subgizmo::SubGizmo;
use crate::{GizmoDirection, GizmoMode, GizmoResult, Ray};

/// Picks given translation subgizmo. If the subgizmo is close enough to
/// the mouse pointer, distance from camera to the subgizmo is returned.
pub(crate) fn pick_translation(subgizmo: &SubGizmo, ray: Ray) -> Option<f32> {
    let origin = subgizmo.config.translation;
    let dir = subgizmo.normal();
    let scale = subgizmo.config.scale_factor * subgizmo.config.visuals.gizmo_size;
    let length = scale;
    let ray_length = 10000.0;

    let (ray_t, subgizmo_t) = segment_to_segment(
        ray.origin,
        ray.origin + ray.direction * ray_length,
        origin,
        origin + dir * length,
    );

    let ray_point = ray.origin + ray.direction * ray_length * ray_t;
    let subgizmo_point = origin + dir * length * subgizmo_t;
    let dist = (ray_point - subgizmo_point).length();

    subgizmo.update_state_with(|state| {
        state.focused = false;
        state.translation.start_point = subgizmo_point;
        state.translation.last_point = subgizmo_point;
        state.translation.current_delta = Vec3::ZERO;
    });

    if dist <= subgizmo.config.focus_distance {
        Some(ray.origin.distance(ray_point))
    } else {
        None
    }
}

pub(crate) fn draw_translation(subgizmo: &SubGizmo) {
    let transform = if subgizmo.config.local_space() {
        Mat4::from_rotation_translation(subgizmo.config.rotation, subgizmo.config.translation)
    } else {
        Mat4::from_translation(subgizmo.config.translation)
    };

    let painter = Painter3d::new(
        subgizmo.ui.painter().clone(),
        subgizmo.config.view_projection * transform,
        subgizmo.config.viewport,
    );

    let direction = subgizmo.local_normal();

    let state = subgizmo.state();

    let color = if state.focused {
        subgizmo
            .config
            .visuals
            .highlight_color
            .unwrap_or_else(|| subgizmo.color())
    } else {
        subgizmo.color()
    };

    let alpha = if state.focused {
        subgizmo.config.visuals.highlight_alpha
    } else {
        subgizmo.config.visuals.inactive_alpha
    };

    let color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let width = subgizmo.config.scale_factor * subgizmo.config.visuals.stroke_width;
    let length = subgizmo.config.scale_factor * subgizmo.config.visuals.gizmo_size;
    let arrow_half_width = width * 2.0;
    let arrow_length = arrow_half_width + length * 0.1;
    let length = length - arrow_length;

    let start = direction * width;
    let end = direction * length;

    painter.line_segment(start, end, (subgizmo.config.visuals.stroke_width, color));

    let cross = if subgizmo.config.local_space() {
        direction.cross(subgizmo.config.rotation.inverse() * subgizmo.config.view_forward())
    } else {
        direction.cross(subgizmo.config.view_forward())
    } * arrow_half_width;

    painter.polygon(
        &[end + cross, end - cross, end + (direction * arrow_length)],
        color,
        (0.0, color),
    );
}

/// Updates given translation subgizmo.
/// If the subgizmo is active, returns the translation result.
pub(crate) fn update_translation(
    subgizmo: &SubGizmo,
    ray: Ray,
    interaction: &Response,
) -> Option<GizmoResult> {
    let state = subgizmo.state();

    let mut new_point = point_on_axis(subgizmo, ray);
    let mut new_delta = new_point - state.translation.start_point;

    let delta_length = new_delta.length();
    if subgizmo.config.snapping && delta_length > 1e-5 {
        new_delta = new_delta / delta_length
            * round_to_interval(delta_length, subgizmo.config.snap_distance);
        new_point = state.translation.start_point + new_delta;
    }

    let (scale, rotation, mut translation) =
        subgizmo.config.model_matrix.to_scale_rotation_translation();
    translation += new_point - state.translation.last_point;

    subgizmo.update_state_with(|state| {
        state.active = interaction.dragged_by(PointerButton::Primary);
        state.translation.last_point = new_point;
        state.translation.current_delta = new_delta;
    });

    Some(GizmoResult {
        transform: Mat4::from_scale_rotation_translation(scale, rotation, translation)
            .to_cols_array_2d(),
        mode: GizmoMode::Translate,
        value: state.translation.current_delta.to_array(),
    })
}

/// Picks given translation plane subgizmo. If the subgizmo is close enough to
/// the mouse pointer, distance from camera to the subgizmo is returned.
pub(crate) fn pick_translation_plane(subgizmo: &SubGizmo, ray: Ray) -> Option<f32> {
    let origin = translation_plane_global_origin(subgizmo);

    let normal = subgizmo.normal();

    let (t, dist_from_origin) = ray_to_plane_origin(normal, origin, ray.origin, ray.direction);

    let ray_point = ray.origin + ray.direction * t;

    subgizmo.update_state_with(|state| {
        state.focused = false;
        state.translation.start_point = ray_point;
        state.translation.last_point = ray_point;
        state.translation.current_delta = Vec3::ZERO;
    });

    if dist_from_origin <= translation_plane_size(subgizmo) {
        Some(t)
    } else {
        None
    }
}

pub(crate) fn draw_translation_plane(subgizmo: &SubGizmo) {
    let transform = if subgizmo.config.local_space() {
        Mat4::from_rotation_translation(subgizmo.config.rotation, subgizmo.config.translation)
    } else {
        Mat4::from_translation(subgizmo.config.translation)
    };

    let painter = Painter3d::new(
        subgizmo.ui.painter().clone(),
        subgizmo.config.view_projection * transform,
        subgizmo.config.viewport,
    );

    let state = subgizmo.state();

    let color = if state.focused {
        subgizmo
            .config
            .visuals
            .highlight_color
            .unwrap_or_else(|| subgizmo.color())
    } else {
        subgizmo.color()
    };

    let alpha = if state.focused {
        subgizmo.config.visuals.highlight_alpha
    } else {
        subgizmo.config.visuals.inactive_alpha
    };

    let color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let scale = translation_plane_size(subgizmo) * 0.5;
    let a = translation_plane_binormal(subgizmo.direction) * scale;
    let b = translation_plane_tangent(subgizmo.direction) * scale;

    let origin = translation_plane_local_origin(subgizmo);

    painter.polygon(
        &[
            origin - b - a,
            origin + b - a,
            origin + b + a,
            origin - b + a,
        ],
        color,
        Stroke::none(),
    );
}

/// Updates given translation subgizmo.
/// If the subgizmo is active, returns the translation result.
pub(crate) fn update_translation_plane(
    subgizmo: &SubGizmo,
    ray: Ray,
    interaction: &Response,
) -> Option<GizmoResult> {
    let state = subgizmo.state();

    let mut new_point = point_on_plane(
        subgizmo.normal(),
        translation_plane_global_origin(subgizmo),
        ray,
    )?;
    let mut new_delta = new_point - state.translation.start_point;

    let delta_length = new_delta.length();
    if subgizmo.config.snapping && delta_length > 1e-5 {
        new_delta = new_delta / delta_length
            * round_to_interval(delta_length, subgizmo.config.snap_distance);
        new_point = state.translation.start_point + new_delta;
    }

    let (scale, rotation, mut translation) =
        subgizmo.config.model_matrix.to_scale_rotation_translation();
    translation += new_point - state.translation.last_point;

    subgizmo.update_state_with(|state| {
        state.active = interaction.dragged_by(PointerButton::Primary);
        state.translation.last_point = new_point;
        state.translation.current_delta = new_delta;
    });

    Some(GizmoResult {
        transform: Mat4::from_scale_rotation_translation(scale, rotation, translation)
            .to_cols_array_2d(),
        mode: GizmoMode::Translate,
        value: state.translation.current_delta.to_array(),
    })
}

#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct TranslationState {
    start_point: Vec3,
    last_point: Vec3,
    current_delta: Vec3,
}

fn translation_plane_binormal(direction: GizmoDirection) -> Vec3 {
    match direction {
        GizmoDirection::X => Vec3::Y,
        GizmoDirection::Y => Vec3::X,
        GizmoDirection::Z => Vec3::X,
        GizmoDirection::Screen => Vec3::X, // Unused
    }
}

fn translation_plane_tangent(direction: GizmoDirection) -> Vec3 {
    match direction {
        GizmoDirection::X => Vec3::Z,
        GizmoDirection::Y => Vec3::Z,
        GizmoDirection::Z => Vec3::Y,
        GizmoDirection::Screen => Vec3::X, // Unused
    }
}

fn translation_plane_size(subgizmo: &SubGizmo) -> f32 {
    subgizmo.config.scale_factor
        * (subgizmo.config.visuals.gizmo_size * 0.1 + subgizmo.config.visuals.stroke_width * 2.0)
}

fn translation_plane_local_origin(subgizmo: &SubGizmo) -> Vec3 {
    let offset = subgizmo.config.scale_factor * subgizmo.config.visuals.gizmo_size * 0.4;

    let a = translation_plane_binormal(subgizmo.direction);
    let b = translation_plane_tangent(subgizmo.direction);
    (a + b) * offset
}

fn translation_plane_global_origin(subgizmo: &SubGizmo) -> Vec3 {
    let mut origin = translation_plane_local_origin(subgizmo);
    if subgizmo.config.local_space() {
        origin = subgizmo.config.rotation * origin;
    }
    origin + subgizmo.config.translation
}

/// Finds the nearest point on line that points in translation subgizmo direction
fn point_on_axis(subgizmo: &SubGizmo, ray: Ray) -> Vec3 {
    let origin = subgizmo.config.translation;
    let direction = subgizmo.normal();

    let (_ray_t, subgizmo_t) = ray_to_ray(ray.origin, ray.direction, origin, direction);

    origin + direction * subgizmo_t
}

fn point_on_plane(plane_normal: Vec3, plane_origin: Vec3, ray: Ray) -> Option<Vec3> {
    let mut t = 0.0;
    if !intersect_plane(
        plane_normal,
        plane_origin,
        ray.origin,
        ray.direction,
        &mut t,
    ) {
        None
    } else {
        Some(ray.origin + ray.direction * t)
    }
}
