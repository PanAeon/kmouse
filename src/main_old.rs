use std::os::unix::prelude::AsRawFd;
use std::time::Duration;
use std::thread;
use input::event::keyboard::{KeyboardKeyEvent, KeyboardEventTrait};
use nix::poll::{poll, PollFd, EventFlags};
use uinput::event::controller::Controller::Mouse;
use uinput::event::controller::Mouse::Left;
use uinput::event::Event::{Controller, Relative};
use uinput::event::relative::Position::{X, Y};
use uinput::event::relative::Relative::Position;

use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::{RawFd, FromRawFd, IntoRawFd}};
use std::path::Path;

use input::{Libinput, LibinputInterface};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};


struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<RawFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into_raw_fd())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: RawFd) {
        unsafe {
            File::from_raw_fd(fd);
        }
    }
}

fn main() {
	let mut device = uinput::default().unwrap()
		.name("test").unwrap()
		.event(Controller(Mouse(Left))).unwrap() // It's necessary to enable any mouse button. Otherwise Relative events would not work.
		.event(Relative(Position(X))).unwrap()
		.event(Relative(Position(Y))).unwrap()
		.create().unwrap();

    let mut input = Libinput::new_with_udev(Interface);
    input.udev_assign_seat("seat0").unwrap();
    //input.udev_assign_seat(seat_id);
    let pollfd = PollFd::new(input.as_raw_fd().as_raw_fd(), EventFlags::POLLIN);
     while poll(&mut [pollfd], -1).is_ok() {
         input.dispatch().unwrap();
         for event in &mut input {
            if let input::Event::Keyboard(kbd_event) = event {
                if let input::event::KeyboardEvent::Key(e) = kbd_event {
                    let key = e.key();
                    if key == 70 {
                    println!("Got event: {:?}", key);

                    device.send(X, 50).unwrap();
                    device.send(Y, 50).unwrap();
                    device.synchronize().unwrap();
                    }
                }
            }
             // do some processing...
         }
    }
}

/*fn fain() {
	for _ in 1..10 {
		thread::sleep(Duration::from_secs(1));

		device.send(X, 50).unwrap();
		device.send(Y, 50).unwrap();
		device.synchronize().unwrap();
	}
}*/
