use slint::{SharedString, Timer, TimerMode, PhysicalPosition};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

// Include the generated Slint code
slint::include_modules!();

#[derive(Clone, Copy, PartialEq)]
enum TimerState {
    Stopped,
    Running,
    Paused,
}

// Window drag state
struct DragState {
    is_dragging: bool,
    start_pos: (f32, f32),
    window_start_pos: Option<PhysicalPosition>,
}

struct PomodoroApp {
    state: Rc<RefCell<TimerState>>,
    window: PomodoroWindow,
    timer: Timer,
    work_duration: Duration,
    break_duration: Duration,
    remaining_time: Rc<RefCell<Duration>>,
    drag_state: Rc<RefCell<DragState>>,
}

impl PomodoroApp {
    fn new() -> Self {
        let window = PomodoroWindow::new().unwrap();
        let work_duration = Duration::from_secs(25 * 60); // 25 minutes
        let break_duration = Duration::from_secs(5 * 60); // 5 minutes
        let remaining_time = Rc::new(RefCell::new(work_duration));

        let state = Rc::new(RefCell::new(TimerState::Stopped));
        let remaining_time_clone = remaining_time.clone();
        let window_clone_weak = window.as_weak();

        // Update display every second when running
        let timer = Timer::default();
        let state_clone = state.clone();

        timer.start(TimerMode::Repeated, std::time::Duration::from_secs(1), move || {
            if *state_clone.borrow() == TimerState::Running {
                let mut remaining = remaining_time_clone.borrow_mut();
                if remaining.as_secs() > 0 {
                    *remaining = remaining.saturating_sub(Duration::from_secs(1));
                }
                // Copy the time before dropping the mutable borrow
                let time = *remaining;
                drop(remaining);

                if let Some(window) = window_clone_weak.upgrade() {
                    let minutes = time.as_secs() / 60;
                    let seconds = time.as_secs() % 60;
                    window.set_display_text(SharedString::from(format!(
                        "{:02}:{:02}",
                        minutes, seconds
                    )));
                    window.set_time_left(time.as_millis() as i64);
                }
            }
        });

        PomodoroApp {
            state,
            window,
            timer,
            work_duration,
            break_duration,
            remaining_time,
            drag_state: Rc::new(RefCell::new(DragState {
                is_dragging: false,
                start_pos: (0.0, 0.0),
                window_start_pos: None,
            })),
        }
    }

    fn start(&self) {
        *self.state.borrow_mut() = TimerState::Running;
        self.window.set_is_running(true);
    }

    fn pause(&self) {
        *self.state.borrow_mut() = TimerState::Paused;
        self.window.set_is_running(false);
    }

    fn reset(&self) {
        *self.state.borrow_mut() = TimerState::Stopped;
        let is_break = self.window.get_is_break();
        let duration = if is_break {
            self.break_duration
        } else {
            self.work_duration
        };
        *self.remaining_time.borrow_mut() = duration;
        self.update_display();
        self.window.set_is_running(false);
    }

    fn toggle_break(&self) {
        let is_break = self.window.get_is_break();
        self.window.set_is_break(!is_break);
        self.reset();
    }

    fn update_display(&self) {
        let time = *self.remaining_time.borrow();
        let minutes = time.as_secs() / 60;
        let seconds = time.as_secs() % 60;
        self.window
            .set_display_text(SharedString::from(format!("{:02}:{:02}", minutes, seconds)));
        self.window.set_time_left(time.as_millis() as i64);
    }

    fn run(self) -> Result<(), slint::PlatformError> {
        // Connect callbacks
        let app = Rc::new(self);

        let app_clone = app.clone();
        app.window.on_start(move || {
            app_clone.start();
        });

        let app_clone = app.clone();
        app.window.on_pause(move || {
            app_clone.pause();
        });

        let app_clone = app.clone();
        app.window.on_reset(move || {
            app_clone.reset();
        });

        let app_clone = app.clone();
        app.window.on_toggle_break(move || {
            app_clone.toggle_break();
        });

        // Connect drag callbacks
        let app_clone = app.clone();
        app.window.on_begin_drag(move |_x, _y| {
            let mut drag_state = app_clone.drag_state.borrow_mut();
            drag_state.is_dragging = true;
            drag_state.window_start_pos = Some(app_clone.window.window().position());
        });

        let app_clone = app.clone();
        app.window.on_continue_drag(move |dx, dy| {
            let drag_state = app_clone.drag_state.borrow();
            if !drag_state.is_dragging {
                return;
            }
            if let Some(start_pos) = drag_state.window_start_pos {
                let new_x = start_pos.x + dx as i32;
                let new_y = start_pos.y + dy as i32;
                drop(drag_state);
                app_clone.window.window().set_position(PhysicalPosition::new(new_x, new_y));
            }
        });

        let app_clone = app.clone();
        app.window.on_end_drag(move || {
            let mut drag_state = app_clone.drag_state.borrow_mut();
            drag_state.is_dragging = false;
            drag_state.window_start_pos = None;
        });

        // Initialize display
        app.update_display();

        app.window.run()
    }
}

fn main() -> Result<(), slint::PlatformError> {
    PomodoroApp::new().run()
}