use cosmic::iced::mouse;
use cosmic::iced::widget::canvas::{self, Canvas, Geometry, Path};
use cosmic::iced::{Color, Point, Rectangle, Size, Vector};
use cosmic::Element;

const CANVAS_INSET: f32 = 1.0;
const SNAP_SCALE: f32 = 2.0; // snap to half-pixels

fn snap_f(value: f32) -> f32 {
    (value * SNAP_SCALE).round() / SNAP_SCALE
}

fn snap_point(point: Point) -> Point {
    Point::new(snap_f(point.x), snap_f(point.y))
}

fn clamp_radius_inside_circle(
    container_center: Point,
    container_radius: f32,
    shape_center: Point,
    desired_radius: f32,
) -> f32 {
    let dx = shape_center.x - container_center.x;
    let dy = shape_center.y - container_center.y;
    let max = (container_radius - (dx * dx + dy * dy).sqrt()).max(0.0);
    desired_radius.min(max)
}

#[derive(Debug)]
pub struct Eyes {
    left_pupil_offset: Vector,
    right_pupil_offset: Vector,
    cursor_known: bool,
}

#[derive(Debug, Clone, Copy)]
struct Layout {
    eye_radius: f32,
    pupil_radius: f32,
    eye_spacing: f32,
}

impl Eyes {
    pub fn new() -> Self {
        Self {
            left_pupil_offset: Vector::new(0.0, 0.0),
            right_pupil_offset: Vector::new(0.0, 0.0),
            cursor_known: false,
        }
    }

    pub fn tick(&mut self, cursor: Option<Point>, window_size: Size, dt: f32) {
        self.cursor_known = cursor.is_some();
        let (layout, left_eye_center, right_eye_center) = layout_and_centers(window_size);

        // If we don't have pointer position, keep pupils centered (not looking inward).
        let left_target = cursor.unwrap_or(left_eye_center);
        let right_target = cursor.unwrap_or(right_eye_center);

        self.left_pupil_offset = pupil_offset(
            left_eye_center,
            left_target,
            self.left_pupil_offset,
            layout,
            dt,
        );
        self.right_pupil_offset = pupil_offset(
            right_eye_center,
            right_target,
            self.right_pupil_offset,
            layout,
            dt,
        );
    }

    pub(crate) fn view(&self) -> Element<'_, crate::Message> {
        Canvas::<&Eyes, crate::Message, cosmic::Theme, cosmic::Renderer>::new(self)
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .into()
    }
}

fn layout_for(size: Size) -> Layout {
    let w = size.width.max(1.0);
    let h = size.height.max(1.0);

    let min_dim = w.min(h);
    // Round to whole logical pixels so the two eyes land on stable subpixel positions.
    let eye_spacing = (min_dim * 0.12).clamp(2.0, 10.0).round();

    // Two eyes side-by-side: total width is roughly `4r + spacing`.
    let max_r_by_width = ((w - eye_spacing) / 4.0).max(1.0);
    let max_r_by_height = (h * 0.45).max(1.0);

    let eye_radius = snap_f(max_r_by_width.min(max_r_by_height).clamp(6.0, 32.0));
    let pupil_radius =
        snap_f((eye_radius * 0.38).clamp(2.0, eye_radius.max(2.0) - 1.0));

    Layout { eye_radius, pupil_radius, eye_spacing }
}

fn layout_and_centers(size: Size) -> (Layout, Point, Point) {
    let inset = CANVAS_INSET;
    let w = (size.width - inset * 2.0).max(1.0);
    let h = (size.height - inset * 2.0).max(1.0);

    let usable_size = Size::new(w, h);
    let layout = layout_for(usable_size);

    let center = Point::new(inset + usable_size.width / 2.0, inset + usable_size.height / 2.0);
    let left_eye_center =
        Point::new(center.x - layout.eye_radius - layout.eye_spacing / 2.0, center.y);
    let right_eye_center =
        Point::new(center.x + layout.eye_radius + layout.eye_spacing / 2.0, center.y);

    (layout, left_eye_center, right_eye_center)
}

fn pupil_offset(
    eye_center: Point,
    cursor: Point,
    current: Vector,
    layout: Layout,
    dt: f32,
) -> Vector {
    let vx = cursor.x - eye_center.x;
    let vy = cursor.y - eye_center.y;

    let distance = (vx * vx + vy * vy).sqrt();
    let max = (layout.eye_radius - layout.pupil_radius).max(0.0);

    let target = if distance > max && distance > 0.0 {
        let scale = max / distance;
        Vector::new(vx * scale, vy * scale)
    } else {
        Vector::new(vx, vy)
    };

    let t = (12.0 * dt).clamp(0.0, 1.0);
    Vector::new(
        current.x + (target.x - current.x) * t,
        current.y + (target.y - current.y) * t,
    )
}

impl canvas::Program<crate::Message, cosmic::Theme, cosmic::Renderer> for &Eyes {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (layout, left_eye_center, right_eye_center) = layout_and_centers(bounds.size());

        let sclera = if self.cursor_known {
            Color::from_rgb8(250, 250, 250)
        } else {
            Color::from_rgb8(160, 190, 255)
        };
        let outline = Color::from_rgb8(24, 24, 24);
        let shadow = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.12 };
        let highlight = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.22 };
        let pupil = Color::from_rgb8(12, 12, 12);
        let pupil_highlight = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.30 };

        let mut draw_eye = |center: Point, pupil_offset: Vector| {
            let center = snap_point(center);
            let pupil_center = center + pupil_offset;
            let inner_r = (layout.eye_radius - 1.0).max(0.0);

            frame.fill(
                &Path::circle(center, layout.eye_radius),
                sclera,
            );

            frame.stroke(
                &Path::circle(center, layout.eye_radius - 0.5),
                canvas::Stroke::default().with_width(1.0).with_color(outline),
            );

            // These highlights must stay inside the sclera (Canvas has no path-clip),
            // otherwise semi-transparent pixels "bleed" outside the outline.
            let shadow_center = snap_point(Point::new(
                center.x + layout.eye_radius * 0.10,
                center.y + layout.eye_radius * 0.12,
            ));
            let shadow_radius = clamp_radius_inside_circle(
                center,
                inner_r,
                shadow_center,
                layout.eye_radius * 0.78,
            );
            if shadow_radius > 0.0 {
                frame.fill(&Path::circle(shadow_center, shadow_radius), shadow);
            }

            frame.fill(
                &Path::circle(
                    snap_point(Point::new(
                        center.x - layout.eye_radius * 0.18,
                        center.y - layout.eye_radius * 0.22,
                    )),
                    clamp_radius_inside_circle(
                        center,
                        inner_r,
                        snap_point(Point::new(
                            center.x - layout.eye_radius * 0.18,
                            center.y - layout.eye_radius * 0.22,
                        )),
                        layout.eye_radius * 0.55,
                    ),
                ),
                highlight,
            );

            frame.fill(&Path::circle(pupil_center, layout.pupil_radius), pupil);
            frame.fill(
                &Path::circle(
                    Point::new(
                        pupil_center.x - layout.pupil_radius * 0.35,
                        pupil_center.y - layout.pupil_radius * 0.35,
                    ),
                    (layout.pupil_radius * 0.28).max(1.0),
                ),
                pupil_highlight,
            );
        };

        draw_eye(left_eye_center, self.left_pupil_offset);
        draw_eye(right_eye_center, self.right_pupil_offset);

        vec![frame.into_geometry()]
    }
}
