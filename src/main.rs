use std::time::{Duration, SystemTime};

use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, Key, RelativeAxisType, InputEvent, EventType, InputEventKind, EventStream};
use tokio::time;

// Acceleration Mode
// https://en.wikipedia.org/wiki/Mouse_keys#MouseKeysAccel
static MK_ACTION_DELTA: i32 = 7;
static MK_DELAY: u64 = 32; 	//milliseconds between the initial key press and first repeated motion event
static MK_INTERVAL: u64 = 16; // 	milliseconds between repeated motion events
static MK_MAX_SPEED: i32 = 10; 	// steady speed (in action_delta units) applied each event
static MK_TIME_TO_MAX: u64 = 40; 	// number of events (count) accelerating to steady speed
static MK_CURVE: i32 = 100; //	ramp used to reach maximum pointer speed
                            //

// Kinetic mode
// https://docs.qmk.fm/#/feature_mouse_keys?id=kinetic-mode
static MOUSEKEY_DELAY: u64 = 5; //  	Delay between pressing a movement key and cursor movement
static MOUSEKEY_INTERVAL: u64 =  8; // 	Time between cursor movements in milliseconds
static MOUSEKEY_MOVE_DELTA: i32 = 16; //	Step size for accelerating from initial to base speed
static MOUSEKEY_INITIAL_SPEED: i32 = 100; // 	Initial speed of the cursor in pixel per second
static MOUSEKEY_BASE_SPEED: i32 =  	5000; // 	Maximum cursor speed at which acceleration stops
static MOUSEKEY_DECELERATED_SPEED: i32 = 	400; //	Decelerated cursor speed
static MOUSEKEY_ACCELERATED_SPEED: i32 =  	3000; // 	Accelerated cursor speed




pub fn pick_device() -> evdev::Device {
    use std::io::prelude::*;

    let mut args = std::env::args_os();
    args.next();
    if let Some(dev_file) = args.next() {
        evdev::Device::open(dev_file).unwrap()
    } else {
        let mut devices = evdev::enumerate().map(|t| t.1).collect::<Vec<_>>();
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let d = pick_device();
    println!("{}", d);

    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);
    keys.insert(Key::BTN_RIGHT);

    let mut axes = AttributeSet::<RelativeAxisType>::new();
    axes.insert(RelativeAxisType::REL_X);
    axes.insert(RelativeAxisType::REL_Y);
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
    let mut num_repeat: u64 = 0;
    let mut action: u64 = 0;

    let mut right_pressed = false;
    let mut left_pressed = false;
    let mut up_pressed = false;
    let mut down_pressed = false;

    let mut left_button_down = false;

    let mut interval = time::interval(Duration::from_millis(MK_INTERVAL));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    let mut events = d.into_event_stream()?;
    loop {
        let maybe_event : Option<InputEvent> = tokio::select!  {
            e = wait_for_input(&mut events) => Some(e?),
            _ = async {
                if right_pressed || left_pressed || up_pressed || down_pressed {
                    interval.tick().await;
                } else {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            } => None
            
        };
        if let Some(ev) = maybe_event {
            // event
            // mouse 17, 18, 19
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
                    }

                } else {
                    up_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                        num_repeat = 0;
                    }
                }
            },
            InputEventKind::Key(Key::KEY_F16)  => {
                if ev.value() == 1 {
                    right_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                      movement_start_time = ev.timestamp();
                    }

                } else {
                    right_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                        num_repeat = 0;
                    }
                }

            },
            InputEventKind::Key(Key::KEY_F14)  => {
                if ev.value() == 1 {
                    down_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                      movement_start_time = ev.timestamp();
                    }

                } else {
                    down_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                        num_repeat = 0;
                    }
                }

            },
            InputEventKind::Key(Key::KEY_F13)  => {
                if ev.value() == 1 {
                    left_pressed = true;
                    if movement_start_time == SystemTime::UNIX_EPOCH {
                        movement_start_time = ev.timestamp();
                    }

                } else {
                    left_pressed = false;
                    if !(up_pressed || down_pressed || left_pressed || right_pressed) {
                        movement_start_time = SystemTime::UNIX_EPOCH;
                        num_repeat = 0;
                    }
                }

            },
            _ => {}
        }
        //println!("{:?}", ev);
        } else {
            let mut events: Vec<InputEvent> = vec![];
            let repeat_delay = time::Duration::from_millis(MK_DELAY);
            
            // TODO: emit side moves
            if right_pressed && (num_repeat != 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                //let d = curve(movement_start_time);
                let d = mouse_keys_accel(num_repeat);
                let move_right = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, d);
                events.push(move_right);
            }
            if left_pressed  && (num_repeat != 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                // let d = curve(movement_start_time);
                let d = mouse_keys_accel(num_repeat);
                let move_left = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, -d);
                events.push(move_left);
            }
            if up_pressed  && (num_repeat != 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
                //let d = curve(movement_start_time);
                let d = mouse_keys_accel(num_repeat);
                let move_up = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, -d);
                events.push(move_up);
            }
            if down_pressed  && (num_repeat != 0 || movement_start_time.elapsed().unwrap() > repeat_delay) {
              //let d = curve(movement_start_time);
              let d = mouse_keys_accel(num_repeat);
              let move_down = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, d);
              events.push(move_down);
            }
            if !events.is_empty() {
                num_repeat += 1;
                device.emit(&events).unwrap();
            }
            // timer tick
        }
    }
}


async fn wait_for_input(events: &mut EventStream) -> Result<InputEvent, Box<dyn std::error::Error>>  {
    Ok(events.next_event().await?)
}

// return action accordingly
fn mouse_keys_accel(i: u64) -> i32 {
  if i == 0 {
     MK_ACTION_DELTA
  } else if i >= MK_TIME_TO_MAX {
      MK_MAX_SPEED * MK_ACTION_DELTA 
  } else {
      let action = MK_ACTION_DELTA as f32 * 
          MK_MAX_SPEED as f32 * ((i as f32 / MK_TIME_TO_MAX as f32).powf((1000.0 + MK_CURVE as f32) / 1000.0));
      action.floor() as i32
  }
}
// todo: maybe use acceleration formula
fn curve(t: SystemTime) -> i32 {
              let elapsed =  t.elapsed().unwrap().as_millis(); // 10ms resolution 
              if elapsed < 150 {
                  1
              } else if elapsed < 250 {
                  2
              } else if elapsed < 350 {
                  3
              } else if elapsed < 500 {
                  4
              } else if elapsed < 600 {
                  5
              } else if elapsed < 700 {
                  6
              } else if elapsed < 800 {
                  7
              } else if elapsed < 900 {
                  10 
              } else if elapsed < 1000 {
                  12
              } else {
                  16
              }

}



