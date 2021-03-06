use core::{fmt, ptr::NonNull};
use gfxconsole::{Bgr32, Format, Framebuffer, GfxConsole, Pixel, Rgb32};
use hal::boot_info::VideoModeInfo;
use hal_x86_64::hw::serial::SerialPort;
use log::{LevelFilter, Log, Metadata, Record};
use spin::Mutex;
use uefi::proto::console::text::Output;

pub static LOGGER: Mutex<Logger> = Mutex::new(Logger::Nop);

struct LogWrapper;

impl Log for LogWrapper {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        use core::fmt::Write;

        if self.enabled(record.metadata()) {
            LOGGER.lock().write_fmt(format_args!("[{}] {}\n", record.level(), record.args())).unwrap();
        }
    }

    fn flush(&self) {}
}

pub enum Logger {
    Nop,
    Console(NonNull<Output<'static>>),
    Serial(SerialPort),
    /*
     * To avoid putting a generic on `Logger` (which is difficult to work with as it's stored in a static), we
     * enumerate each possible pixel format here. Kinda sucks, but it's the best way I've found to do this.
     */
    Rgb32 { serial_port: SerialPort, console: GfxConsole<gfxconsole::Rgb32> },
    Bgr32 { serial_port: SerialPort, console: GfxConsole<gfxconsole::Bgr32> },
}

impl Logger {
    pub fn init_console(console_writer: &mut Output) {
        *LOGGER.lock() = Logger::Console(NonNull::new(console_writer as *const _ as *mut _).unwrap());

        log::set_logger(&LogWrapper).unwrap();
        log::set_max_level(LevelFilter::Trace);
    }

    pub fn switch_to_serial() {
        *LOGGER.lock() = Logger::Serial(unsafe { SerialPort::new(hal_x86_64::hw::serial::COM1) });
    }

    pub fn switch_to_gfx(video_mode: &VideoModeInfo) {
        match video_mode {
            VideoModeInfo { framebuffer_address, pixel_format, width, height, stride } => match pixel_format {
                hal::boot_info::PixelFormat::RGB32 => {
                    let framebuffer = Framebuffer {
                        ptr: usize::from(*framebuffer_address) as *mut Pixel<Rgb32>,
                        width: *width,
                        height: *height,
                        stride: *stride,
                    };
                    *LOGGER.lock() = Logger::Rgb32 {
                        serial_port: unsafe { SerialPort::new(hal_x86_64::hw::serial::COM1) },
                        console: GfxConsole::new(
                            framebuffer,
                            Rgb32::pixel(0x00, 0x00, 0xaa, 0xff),
                            Rgb32::pixel(0xff, 0xff, 0xff, 0xff),
                        ),
                    };
                }
                hal::boot_info::PixelFormat::BGR32 => {
                    let framebuffer = Framebuffer {
                        ptr: usize::from(*framebuffer_address) as *mut Pixel<Bgr32>,
                        width: *width,
                        height: *height,
                        stride: *stride,
                    };
                    *LOGGER.lock() = Logger::Bgr32 {
                        serial_port: unsafe { SerialPort::new(hal_x86_64::hw::serial::COM1) },
                        console: GfxConsole::new(
                            framebuffer,
                            Bgr32::pixel(0x00, 0x00, 0xaa, 0xff),
                            Bgr32::pixel(0xff, 0xff, 0xff, 0xff),
                        ),
                    };
                }
            },
        }
    }
}

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        match self {
            Logger::Nop => Ok(()),
            Logger::Console(output) => unsafe { output.as_mut() }.write_str(s),
            Logger::Serial(serial) => serial.write_str(s),
            Logger::Rgb32 { serial_port, console } => {
                serial_port.write_str(s)?;
                console.write_str(s)
            }
            Logger::Bgr32 { serial_port, console } => {
                serial_port.write_str(s)?;
                console.write_str(s)
            }
        }
    }
}

unsafe impl Sync for Logger {}
unsafe impl Send for Logger {}
