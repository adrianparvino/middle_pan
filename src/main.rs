use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xinput::XIEventMask;
use x11rb::protocol::xinput::DeviceUse;
use x11rb::protocol::Event;
use x11rb::protocol::xtest::fake_input;
use anyhow::Result;

const SENSITIVITY: i32 = 4;

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

    let mut x = None;
    let mut y = None;

    loop {
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
            x = Some(true);
            fake_input(&conn, 2, 114, 0, root, 0, 0, 2)?.check()?;

            delta.0 -= SENSITIVITY;
        } else if delta.0 < -SENSITIVITY {
            x = Some(false);
            fake_input(&conn, 2, 113, 0, root, 0, 0, 2)?.check()?;

            delta.0 += SENSITIVITY;
        } else {
            match x {
                Some(true) => {
                    fake_input(&conn, 3, 114, 0, root, 0, 0, 2)?.check()?;
                },
                Some(false) => {
                    fake_input(&conn, 3, 113, 0, root, 0, 0, 2)?.check()?;
                }
                _ => ()
            }
        }

        if delta.1 > SENSITIVITY {
            y = Some(true);
            fake_input(&conn, 2, 116, 0, root, 0, 0, 2)?.check()?;

            delta.1 -= SENSITIVITY;
        } else if delta.1 < -SENSITIVITY {
            y = Some(false);
            fake_input(&conn, 2, 111, 0, root, 0, 0, 2)?.check()?;

            delta.1 += SENSITIVITY;
        } else {
            match y {
                Some(true) => {
                    fake_input(&conn, 3, 116, 0, root, 0, 0, 2)?.check()?;
                },
                Some(false) => {
                    fake_input(&conn, 3, 111, 0, root, 0, 0, 2)?.check()?;
                }
                _ => ()
            }
        }
    }
}
