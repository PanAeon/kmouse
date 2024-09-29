use std::time::{Duration, SystemTime};

use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, Key, RelativeAxisType, InputEvent, EventType, InputEventKind, EventStream};
use tokio::time;

// Acceleration Mode
// https://en.wikipedia.org/wiki/Mouse_keys#MouseKeysAccel
static MK_ACTION_DELTA: i32 = 7;
static MK_DELAY: u64 = 32; 	//milliseconds between the initial key press and first repeated motion event
static MK_INTERVAL: u64 = 8; // 	milliseconds between repeated motion events
static MK_MAX_SPEED: i32 = 10; 	// steady speed (in action_delta units) applied each event
static MK_TIME_TO_MAX: u64 = 120; 	// number of events (count) accelerating to steady speed
static MK_CURVE: i32 = 500; //	ramp used to reach maximum pointer speed

static MK_WHEEL_DELAY: u64 = 10; 	//milliseconds between the initial key press and first repeated motion event
static MK_WHEEL_DELTA: i32 = 3; // hmm.
static MK_WHEEL_INTERVAL: u64 = 80; // hmm.
static MK_WHEEL_MAX_SPEED: u64 = 8;
static MK_WHEEL_TIME_TO_MAX: u64 = 80;
// Kinetic mode
// https://docs.qmk.fm/#/feature_mouse_keys?id=kinetic-mode
/*static MOUSEKEY_DELAY: u64 = 5; //  	Delay between pressing a movement key and cursor movement
static MOUSEKEY_INTERVAL: u64 =  10; // 	Time between cursor movements in milliseconds
static MOUSEKEY_MOVE_DELTA: f32 = 16.0; //	Step size for accelerating from initial to base speed
static MOUSEKEY_INITIAL_SPEED: f32 = 100.0; // 	Initial speed of the cursor in pixel per second
static MOUSEKEY_BASE_SPEED: f32 =  	5000.0; // 	Maximum cursor speed at which acceleration stops
*/




pub fn pick_device() -> evdev::Device {
    use std::io::prelude::*;

    let mut args = std::env::args_os();
    args.next();
    if let Some(dev_file) = args.next() {
        evdev::Device::open(dev_file).unwrap()
    } else {
        let mut devices = evdev::enumerate().map(|t| t.1).collect::<Vec<_>>();
        let maybe_kmonad = devices.iter().enumerate().find(|d| d.1.name() == Some("KMonad output"));
        if let Some((n, _)) = maybe_kmonad {
            devices.into_iter().nth(n).unwrap()

        } else {
        // readdir returns them in reverse order from their eventN names for some reason
        devices.reverse();
        for (i, d) in devices.iter().enumerate() {
            println!("{}: {}", i, d.name().unwrap_or("Unnamed device"));
        }
        print!("Select the device [0-{}]: ", devices.len());
        let _ = std::io::stdout().flush();
        let mut chosen = String::new();
        std::io::stdin().read_line(&mut chosen).unwrap();
        let n = chosen.trim().parse::<usize>().unwrap();
        devices.into_iter().nth(n).unwrap()
        }
    }
}
// 
// libinput debug-events
//
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let d = pick_device();
    println!("{:?}", d.input_id());

    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);
    keys.insert(Key::BTN_RIGHT);

    let mut axes = AttributeSet::<RelativeAxisType>::new();
    axes.insert(RelativeAxisType::REL_X);
    axes.insert(RelativeAxisType::REL_Y);
    axes.insert(RelativeAxisType::REL_WHEEL);
    // it's working! but not in sway yet.., which means I need to send also REL_WHEEL every
    // 120 notches
    //axes.insert(RelativeAxisType::REL_WHEEL_HI_RES); //= WHEEL * 120
    /*
    let abs_x = UinputAbsSetup::new(
        AbsoluteAxisType::ABS_X,
        AbsInfo::new(256, 0, 512, 20, 20, 1)
    );
    let abs_y = UinputAbsSetup::new(
        AbsoluteAxisType::ABS_X,
        AbsInfo::new(256, 0, 512, 20, 20, 1)
    );
    */


    let mut device = VirtualDeviceBuilder::new()?
        .name("KMouse")
        .with_relative_axes(&axes)?
       // .with_absolute_axis(&abs_x)?
       // .with_absolute_axis(&abs_y)?
        .with_keys(&keys)?
        .build()
        .unwrap();

    for path in device.enumerate_dev_nodes_blocking()? {
        let path = path?;
        println!("Available as {}", path.display());
    }
    let mut movement_start_time: SystemTime = SystemTime::UNIX_EPOCH;
    let mut wheel_start_time: SystemTime = SystemTime::UNIX_EPOCH;

    let mut num_repeat: u64 = 0;
    let mut wheel_num_repeat:  u64 = 0;

    let mut right_pressed = false;
    let mut left_pressed = false;
    let mut up_pressed = false;
    let mut down_pressed = false;

    let mut wheel_up_pressed = false;
    let mut wheel_down_pressed = false;

    let mut left_button_down = false;

    let mut interval = time::interval(Duration::from_millis(MK_INTERVAL));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    let mut wheel_interval = time::interval(Duration::from_millis(MK_WHEEL_INTERVAL));
    wheel_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    let mut events = d.into_event_stream()?;
    loop {
        let maybe_event : Option<InputEvent> = tokio::select!  {
            e = wait_for_input(&mut events) => Some(e?),
            _ = async {
                if right_pressed || left_pressed || up_pressed || down_pressed {
                    interval.tick().await;
                } else if wheel_up_pressed || wheel_down_pressed {
                    wheel_interval.tick().await;
                } else {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            } => None
            
        };
        if let Some(ev) = maybe_event {
            // event
            // mouse 17, 18, 19
            //println!("{:?}", ev);
        match ev.kind() {
            InputEventKind::Key(Key::KEY_F17) if ev.value() == 1 => {
              if !left_button_down {
                let left_click = InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 1);
                device.emit(&[left_click]).unwrap();
                //left_button_down = false;
                time::sleep(Duration::from_millis(32)).await;
              }
              let left_up = InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 0);
              device.emit(&[left_up]).unwrap();
              left_button_down = false;
            },
            InputEventKind::Key(Key::KEY_F18) if ev.value() == 1  => {
              let left_click = InputEvent::new(EventType::KEY, Key::BTN_RIGHT.0, 1);
              device.emit(&[left_click]).unwrap();
              time::sleep(Duration::from_millis(32)).await;
              let left_up = InputEvent::new(EventType::KEY, Key::BTN_RIGHT.0, 0);
              device.emit(&[left_up]).unwrap();
            },
            InputEventKind::Key(Key::KEY_F19) if ev.value() == 1   => {
              if left_button_down {
                  let left_click = InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 0);
                  device.emit(&[left_click]).unwrap();
                  left_button_down = false;
              } else {
                  let left_click = InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 1);
                  device.emit(&[left_click]).unwrap();
                  left_button_down = true;

              }
            },
            InputEventKind::Key(Key::KEY_F15)  => {
                if ev.value() == 1 {
                    up_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                      movement_start_time = ev.timestamp();
                      num_repeat = 0;
                    }

                } else {
                    up_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                    }
                }
            },
            InputEventKind::Key(Key::KEY_F16)  => {
                if ev.value() == 1 {
                    right_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                      movement_start_time = ev.timestamp();
                      num_repeat = 0;
                    }

                } else {
                    right_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                    }
                }

            },
            InputEventKind::Key(Key::KEY_F14)  => {
                if ev.value() == 1 {
                    down_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                      movement_start_time = ev.timestamp();
                      num_repeat = 0;
                    }

                } else {
                    down_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                    }
                }

            },
            InputEventKind::Key(Key::KEY_F13)  => {
                if ev.value() == 1 {
                    left_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                        movement_start_time = ev.timestamp();
                        num_repeat = 0;
                    }

                } else {
                    left_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                    }
                }

            },
            InputEventKind::Key(Key::KEY_F20)  => {
                if ev.value() == 1 {
                    wheel_up_pressed = true;
                    wheel_start_time = ev.timestamp();
                    wheel_num_repeat = 0;

                } else {
                    wheel_up_pressed = false;
                }

            },
            InputEventKind::Key(Key::KEY_F21)  => {
                if ev.value() == 1 {
                    wheel_down_pressed = true;
                    wheel_start_time = ev.timestamp();
                    wheel_num_repeat = 0;

                } else {
                    wheel_down_pressed = false;
                }

            },
            _ => {}
        }
        //println!("{:?}", ev);
        } else {
            let mut events: Vec<InputEvent> = vec![];
            let repeat_delay = time::Duration::from_millis(MK_DELAY);
            let wheel_repeat_delay = time::Duration::from_millis(MK_WHEEL_DELAY);
            
            // TODO: emit side moves
            if right_pressed && (num_repeat == 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                //let d = curve(movement_start_time);
                let mut d = mouse_keys_accel(num_repeat);
                if up_pressed || down_pressed {
                    d = d * 181 / 256;
                    if d == 0 {
                        d = 1;
                    }
                }
                //let d = kinetic_action(movement_start_time);
                let move_right = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, d);
                events.push(move_right);
            }
            if left_pressed  && (num_repeat == 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                // let d = curve(movement_start_time);
                let mut d = mouse_keys_accel(num_repeat);
                if up_pressed || down_pressed {
                    d = d * 181 / 256;
                    if d == 0 {
                        d = 1;
                    }
                }
                //let d = kinetic_action(movement_start_time);
                let move_left = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, -d);
                events.push(move_left);
            }
            if up_pressed  && (num_repeat == 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                //let d = curve(movement_start_time);
                let mut d = mouse_keys_accel(num_repeat);
                if left_pressed || right_pressed {
                    d = d * 181 / 256;
                    if d == 0 {
                        d = 1;
                    }
                }
                //let d = kinetic_action(movement_start_time);
                let move_up = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, -d);
                events.push(move_up);
            }
            if down_pressed  && (num_repeat == 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
              //let d = curve(movement_start_time);
              let mut d = mouse_keys_accel(num_repeat);
                if left_pressed || right_pressed {
                    d = d * 181 / 256;
                    if d == 0 {
                        d = 1;
                    }
                }
              //let d = kinetic_action(movement_start_time);
              let move_down = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, d);
              events.push(move_down);
            }

            if wheel_down_pressed  && (wheel_num_repeat == 0 || wheel_start_time.elapsed().unwrap() > wheel_repeat_delay) {
              let d = wheel_keys_accel(wheel_num_repeat);
              let wheel_down = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, -d);
              //let wheel_down = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL_HI_RES.0, -d * 120);
              events.push(wheel_down);
            }
            if wheel_up_pressed  && (wheel_num_repeat == 0 || wheel_start_time.elapsed().unwrap() > wheel_repeat_delay) {
              let d = wheel_keys_accel(wheel_num_repeat);
              let wheel_up = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, d);
              //let wheel_up = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL_HI_RES.0, d * 120);
              events.push(wheel_up);
            }

            if !events.is_empty() {
                num_repeat += 1;
                wheel_num_repeat += 1;
                device.emit(&events).unwrap();
            }
            // timer tick
        }
    }
}


async fn wait_for_input(events: &mut EventStream) -> Result<InputEvent, Box<dyn std::error::Error>>  {
    Ok(events.next_event().await?)
}

fn mouse_keys_accel(i: u64) -> i32 {
  let r = if i == 0 {
     1
  } else if i >= MK_TIME_TO_MAX {
      MK_MAX_SPEED * MK_ACTION_DELTA 
  } else {
      let action = MK_ACTION_DELTA as f32 * 
          MK_MAX_SPEED as f32 * ((i as f32 / MK_TIME_TO_MAX as f32).powf((1000.0 + MK_CURVE as f32) / 1000.0));
      action.floor() as i32
  };
  if r <= 0 { 1 } else { r }
}

fn wheel_keys_accel(i: u64) -> i32 {
  let r = if i == 0 {
     1 // FIXME: deal with this somehow, MK_WHEEL_DELTA
  } else if i >= MK_WHEEL_TIME_TO_MAX {
      MK_MAX_SPEED * MK_WHEEL_DELTA 
  } else {
      let action = MK_WHEEL_DELTA as f32 * 
          MK_WHEEL_MAX_SPEED as f32 * ((i as f32 / MK_WHEEL_TIME_TO_MAX as f32).powf((1000.0 + MK_CURVE as f32) / 1000.0));
      action.floor() as i32
  };
  if r <= 0 { 1 } else { r }
}

/*
 * Kinetic movement  acceleration algorithm
 *
 *  current speed = I + A * T/50 + A * 0.5 * T^2 | maximum B
 *
 * T: time since the mouse movement started
 * E: mouse events per second (set through MOUSEKEY_INTERVAL, UHK sends 250, the
 *    pro micro on my Signum 3.0 sends only 125!)
 * I: initial speed at time 0
 * A: acceleration
 * B: base mouse travel speed
 */
/*fn kinetic_action(t: SystemTime) -> i32 {
    let time_elapsed =  t.elapsed().unwrap().as_millis() as f32 / 10.0; // 10ms resolution 

    let mut speed   = MOUSEKEY_INITIAL_SPEED + MOUSEKEY_MOVE_DELTA * time_elapsed + MOUSEKEY_MOVE_DELTA * 0.5 * time_elapsed * time_elapsed;

    speed = speed.clamp(1000.0, MOUSEKEY_BASE_SPEED);

    speed /= 1000.0 / MOUSEKEY_INTERVAL as f32;

    speed.floor() as i32

}*/


