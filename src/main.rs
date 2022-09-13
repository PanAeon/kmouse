use std::time::{Duration, SystemTime};

use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, Key, RelativeAxisType, InputEvent, EventType, InputEventKind, EventStream};
use tokio::time;


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

    let mut right_pressed = false;
    let mut right_press_time: SystemTime = SystemTime::UNIX_EPOCH;
    let mut left_pressed = false;
    let mut left_press_time: SystemTime = SystemTime::UNIX_EPOCH;
    let mut up_pressed = false;
    let mut up_press_time: SystemTime = SystemTime::UNIX_EPOCH;
    let mut down_pressed = false;
    let mut down_press_time: SystemTime = SystemTime::UNIX_EPOCH;

    let mut left_button_down = false;

    let mut interval = time::interval(Duration::from_millis(16));
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
                    up_press_time = ev.timestamp();

                } else {
                    up_pressed = false;
                }
            },
            InputEventKind::Key(Key::KEY_F16)  => {
                if ev.value() == 1 {
                    right_pressed = true;
                    right_press_time = ev.timestamp();

                } else {
                    right_pressed = false;
                }

            },
            InputEventKind::Key(Key::KEY_F14)  => {
                if ev.value() == 1 {
                    down_pressed = true;
                    down_press_time = ev.timestamp();

                } else {
                    down_pressed = false;
                }

            },
            InputEventKind::Key(Key::KEY_F13)  => {
                if ev.value() == 1 {
                    left_pressed = true;
                    left_press_time = ev.timestamp();

                } else {
                    left_pressed = false;
                }

            },
            _ => {}
        }
        //println!("{:?}", ev);
        } else {
            let mut events: Vec<InputEvent> = vec![];
            
            // TODO: emit side moves
            if right_pressed {
              let d = curve(right_press_time);
              let move_right = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, d);
              events.push(move_right);
            }
            if left_pressed {
              let d = curve(left_press_time);
              let move_left = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, -d);
              events.push(move_left);
            }
            if up_pressed {
              let d = curve(up_press_time);
              let move_up = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, -d);
              events.push(move_up);
            }
            if down_pressed {
              let d = curve(down_press_time);
              let move_down = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, d);
              events.push(move_down);
            }
            if !events.is_empty() {
                device.emit(&events).unwrap();
            }
            // timer tick
        }
    }
}


// Mouse key Accel:
// https://en.wikipedia.org/wiki/Mouse_keys#MouseKeysAccel
// also: https://github.com/qmk/qmk_firmware/blob/master/docs/feature_mouse_keys.md
// mk_delay 	milliseconds between the initial key press and first repeated motion event
// mk_interval 	milliseconds between repeated motion events
// mk_max_speed 	steady speed (in action_delta units) applied each event
// mk_time_to_max 	number of events (count) accelerating to steady speed
// mk_curve 	ramp used to reach maximum pointer speed


async fn wait_for_input(events: &mut EventStream) -> Result<InputEvent, Box<dyn std::error::Error>>  {
    Ok(events.next_event().await?)
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



