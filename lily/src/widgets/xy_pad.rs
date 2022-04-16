use std::collections::HashMap;

use femtovg::{Paint, Path};
use glam::Vec2;
use lily_derive::Handle;
use vizia::*;

use crate::util::BoundingBoxExt;

/// The distance in pixels before a node is considered hovered
const HOVER_RADIUS: f32 = 16f32;

/// Controls a single point along a normalized XY axis `(-1,-1)..=(1,1)`.
#[derive(Handle)]
pub struct XyPad<P>
where
    P: Lens<Target = Vec2>,
{
    point: P,
    state: InternalState,
    // Temporary workaround until we can get custom css stuff directly
    classes: HashMap<&'static str, Entity>,
    #[callback(Vec2)]
    on_changing_point: Option<Box<dyn Fn(&mut Context, Vec2)>>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum InternalState {
    NoOp,
    Hovering,
    Dragging,
}

enum InternalEvent {
    UpdateState { state: InternalState },
}

impl<P> XyPad<P>
where
    P: Lens<Target = Vec2>,
{
    pub fn new(cx: &mut Context, point: P) -> Handle<Self> {
        let mut classes = HashMap::<&'static str, Entity>::default();
        let mut insert_color = |name| {
            let e = Element::new(cx).class(name).display(Display::None).entity;
            classes.insert(name, e);
        };
        insert_color("point");
        insert_color("crosshair");
        Self {
            point,
            on_changing_point: None,
            state: InternalState::NoOp,
            classes,
        }
        .build(cx, |_| {})
    }
}

impl<P> View for XyPad<P>
where
    P: Lens<Target = Vec2>,
{
    fn element(&self) -> Option<String> {
        Some("xy".to_string())
    }

    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        // If clicking and hovered, set the state to dragging
        if let Some(ev) = event.message.downcast::<InternalEvent>() {
            match ev {
                InternalEvent::UpdateState { state } => self.state = *state,
            }
        }
        if let Some(ev) = event.message.downcast::<WindowEvent>() {
            match ev {
                WindowEvent::MouseMove(x, y) => {
                    let rect = cx.cache.get_bounds(cx.current);
                    let point = self.point.get(cx);
                    let ui_point = rect.map_data_point(point, true);
                    let cursor = Vec2::new(*x, *y);
                    // If within range of the cursor and not currently being dragged, set to being hovered
                    if cursor.distance_squared(ui_point) <= HOVER_RADIUS.powi(2) {
                        if self.state == InternalState::NoOp {
                            cx.emit(InternalEvent::UpdateState {
                                state: InternalState::Hovering,
                            });
                        }
                    } else if self.state != InternalState::Dragging {
                        cx.emit(InternalEvent::UpdateState {
                            state: InternalState::NoOp,
                        });
                    }

                    if let InternalState::Dragging = self.state {
                        if let Some(callback) = &self.on_changing_point {
                            let point = Vec2::new(*x, *y);
                            let point_normalized =
                                cx.cache.get_bounds(cx.current).map_ui_point(point, true);
                            (callback)(cx, point_normalized);
                        }
                    }
                }
                WindowEvent::MouseDown(button) => {
                    if *button == MouseButton::Left {
                        cx.capture();
                        if self.state == InternalState::Hovering {
                            self.state = InternalState::Dragging;
                        }
                    }
                }
                WindowEvent::MouseUp(button) => {
                    if *button == MouseButton::Left {
                        cx.release();
                        self.state = if self.state == InternalState::Dragging {
                            InternalState::Hovering
                        } else {
                            InternalState::NoOp
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let entity = cx.current();
        let cursor = Vec2::new(cx.mouse().cursorx, cx.mouse().cursory);
        let rect = cx.cache().get_bounds(entity);
        let bg = cx.background_color(entity).cloned().unwrap_or_default();
        let border = cx.border_color(entity).cloned().unwrap_or_default();

        // Draw background shapes
        // Background
        let mut path = Path::new();
        path.rect(rect.x, rect.y, rect.w, rect.h);
        canvas.fill_path(&mut path, Paint::color(bg.into()));

        // XY center lines
        let (center_top_x, center_top_y) = rect.center_top();
        let (center_bottom_x, center_bottom_y) = rect.center_bottom();
        let (center_left_x, center_left_y) = rect.center_left();
        let (center_right_x, center_right_y) = rect.center_right();

        let mut path = Path::new();
        path.move_to(center_top_x, center_top_y);
        path.line_to(center_bottom_x, center_bottom_y);
        path.move_to(center_left_x, center_left_y);
        path.line_to(center_right_x, center_right_y);

        // Circle reference lines
        let (center_x, center_y) = rect.center();
        for scale in [1.0, 0.66, 0.33] {
            path.circle(center_x, center_y, (rect.w / 2f32) * scale);
        }
        canvas.stroke_path(&mut path, Paint::color(border.into()));

        // Draw crosshairs when dragging
        let crosshair_entity = *self.classes.get("crosshair").unwrap();
        let crosshair_color = cx
            .border_color(crosshair_entity)
            .cloned()
            .unwrap_or_default();
        if self.state == InternalState::Dragging {
            let mut path = Path::new();
            path.move_to(cursor.x, rect.top());
            path.line_to(cursor.x, rect.bottom());
            path.move_to(rect.left(), cursor.y);
            path.line_to(rect.right(), cursor.y);
            canvas.stroke_path(&mut path, Paint::color(crosshair_color.into()));
        }

        // Data point
        self.point.view(cx.data().unwrap(), |point| {
            let point = *point.unwrap();
            let point_entity = *self.classes.get("point").unwrap();
            let ui_point = rect.map_data_point(point, true);
            let point_border = cx.border_color(point_entity).cloned().unwrap_or_default();

            let point_color = cx
                .background_color(point_entity)
                .cloned()
                .unwrap_or_default();

            // Point fill
            let mut path = Path::new();
            path.circle(ui_point.x, ui_point.y, 4f32);
            canvas.fill_path(&mut path, Paint::color(point_color.into()));

            // Point outline
            let mut path = Path::new();
            match self.state {
                InternalState::Dragging | InternalState::Hovering => {
                    path.circle(ui_point.x, ui_point.y, 8f32)
                }
                _ => (),
            }

            // Get custom CSS info from a display none element

            canvas.stroke_path(
                &mut path,
                Paint::color(point_border.into()).with_line_width(2f32),
            );
        });
    }
}
