use std::os::unix::prelude::AsRawFd;
use std::task::Poll;

use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xinput::XIEventMask;
use x11rb::protocol::xinput::DeviceUse;
use x11rb::protocol::Event;
use x11rb::protocol::xtest::fake_input;
use nix::poll::{poll, PollFd, PollFlags};
use anyhow::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
enum XDirection {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum YDirection {
    Up,
    Down
}

struct MouseHandle<'conn> {
    x: Option<XDirection>,
    y: Option<YDirection>,
    conn: &'conn RustConnection,
    root: u32
}

impl<'conn> MouseHandle<'conn> {
    fn new(conn: &'conn RustConnection, root: u32) -> Self {
        Self {
            x: None,
            y: None,
            conn,
            root
        }
    }

    fn change_x(&mut self, new_x: Option<XDirection>) -> Result<()> {
        if self.x == new_x { return Ok(()); }

        match self.x {
            None => {},
            Some(XDirection::Left) => {
                fake_input(self.conn, 3, 113, 0, self.root, 0, 0, 2)?.check()?;
            }
            Some(XDirection::Right) => {
                fake_input(self.conn, 3, 114, 0, self.root, 0, 0, 2)?.check()?;
            }
        }

        match new_x {
            None => {},
            Some(XDirection::Left) => {
                fake_input(self.conn, 2, 113, 0, self.root, 0, 0, 2)?.check()?;
            }
            Some(XDirection::Right) => {
                fake_input(self.conn, 2, 114, 0, self.root, 0, 0, 2)?.check()?;
            }
        }

        self.x = new_x;

        Ok(())
    }


    fn change_y(&mut self, new_y: Option<YDirection>) -> Result<()> {
        if self.y == new_y { return Ok(()); }

        match self.y {
            None => {},
            Some(YDirection::Up) => {
                fake_input(self.conn, 3, 116, 0, self.root, 0, 0, 2)?.check()?;
            }
            Some(YDirection::Down) => {
                fake_input(self.conn, 3, 111, 0, self.root, 0, 0, 2)?.check()?;
            }
        }

        match new_y {
            None => {},
            Some(YDirection::Up) => {
                fake_input(self.conn, 2, 116, 0, self.root, 0, 0, 2)?.check()?;
            }
            Some(YDirection::Down) => {
                fake_input(self.conn, 2, 111, 0, self.root, 0, 0, 2)?.check()?;
            }
        }

        self.y = new_y;

        Ok(())
    }
}

const SENSITIVITY: i32 = 10;

fn main() -> Result<()> {
    let (conn, screen) = RustConnection::connect(None).unwrap();

    let screen = &conn.setup().roots[screen];
    let root = screen.root;

    // Get registered devices
    let devices = x11rb::protocol::xinput::list_input_devices(&conn)?.reply()?;
    for device in devices.devices {
        // Find keyboards
        if device.device_use == DeviceUse::IS_X_EXTENSION_POINTER {
            // println!("{:?}", device);
            x11rb::protocol::xinput::xi_select_events(&conn, root, &[
                x11rb::protocol::xinput::EventMask {
                    deviceid: device.device_id as u16,
                    mask: vec![u32::from(XIEventMask::MOTION)]
                }
            ])?.check()?;
        }
    }

    let mut last_location = {
        let reply = x11rb::protocol::xproto::query_pointer(&conn, root)?.reply()?;

        (reply.root_x as i32, reply.root_y as i32)
    };
    let mut delta = (0, 0);

    let mut mouse = MouseHandle::new(&conn, root);

    let mut fds = [ PollFd::new(conn.stream().as_raw_fd(), PollFlags::POLLIN) ];

    loop {
        poll(&mut fds, 1);
        if let Ok(Some(event)) = conn.poll_for_event() {
            match event {
                Event::XinputMotion(event) => {
                    let location = (event.root_x >> 16, event.root_y >> 16);
                    delta = (
                        delta.0 + location.0 - last_location.0,
                        delta.1 + location.1 - last_location.1
                    );

                    if event.button_mask[0] & (1 << 9) == 0 {
                        delta = (0, 0);
                    }

                    last_location = location;
                },
                _ => ()
            }
        }

        if delta.0 > SENSITIVITY {
            mouse.change_x(Some(XDirection::Right))?;
            delta.0 -= SENSITIVITY;
        } else if delta.0 < -SENSITIVITY {
            mouse.change_x(Some(XDirection::Left))?;
            delta.0 += SENSITIVITY;
        } else {
            mouse.change_x(None)?;
        }

        if delta.1 > SENSITIVITY {
            mouse.change_y(Some(YDirection::Up))?;
            delta.1 -= SENSITIVITY;
        } else if delta.1 < -SENSITIVITY {
            mouse.change_y(Some(YDirection::Down))?;
            delta.1 += SENSITIVITY;
        } else {
            mouse.change_y(None)?;
        }
    }
}
