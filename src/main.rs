#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod display;

use core::cell::RefCell;

use critical_section::Mutex;
use display::Display;
use embedded_hal_bus::i2c::RefCellDevice;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Pull};
use esp_hal::i2c::master::{self as i2c_master, I2c};
use esp_hal::time::Duration;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::timer::{OneShotTimer, PeriodicTimer};
use esp_hal::{Blocking, handler, main};
use esp_println::println;
use scd4x::Scd4x;
use scd4x::types::SensorData;

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("{}", e);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

static ISR_INPUT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static ISR_SENSOR_TIMER: Mutex<RefCell<Option<PeriodicTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));
static ISR_SCREEN_TIMEOUT: Mutex<RefCell<Option<OneShotTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));

// use Mutex instead of AtomicBool because ESP32C3 does not support AtomicBool::swap
static IS_GPIO_ISR: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static IS_SENSOR_TIMER_ISR: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static IS_SCREEN_TIMEOUT_ISR: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

fn get_and_clear_isr_flag(flag: &Mutex<RefCell<bool>>) -> bool {
    critical_section::with(|cs| {
        let mut flag = flag.borrow_ref_mut(cs);
        let val = *flag;
        *flag = false;
        val
    })
}

#[handler]
fn gpio_isr() {
    critical_section::with(|cs| {
        let mut button = ISR_INPUT.borrow_ref_mut(cs);
        let Some(button) = button.as_mut() else {
            return;
        };
        if !button.is_interrupt_set() {
            return;
        }

        *IS_GPIO_ISR.borrow_ref_mut(cs) = true;

        button.clear_interrupt();
    });
}

#[handler]
fn sensor_timer_isr() {
    critical_section::with(|cs| {
        let mut timer = ISR_SENSOR_TIMER.borrow_ref_mut(cs);
        let Some(timer) = timer.as_mut() else {
            return;
        };

        *IS_SENSOR_TIMER_ISR.borrow_ref_mut(cs) = true;

        timer.clear_interrupt();
    });
}

#[handler]
fn button_timer_isr() {
    critical_section::with(|cs| {
        let mut timer = ISR_SCREEN_TIMEOUT.borrow_ref_mut(cs);
        let Some(timer) = timer.as_mut() else {
            return;
        };

        *IS_SCREEN_TIMEOUT_ISR.borrow_ref_mut(cs) = true;

        timer.clear_interrupt();
    });
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(gpio_isr);

    let input_config = InputConfig::default().with_pull(Pull::Up);
    let mut button = Input::new(peripherals.GPIO2, input_config);
    critical_section::with(|cs| {
        button.listen(Event::FallingEdge);
        ISR_INPUT.borrow_ref_mut(cs).replace(button);
    });

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1);

    let mut sensor_timer = PeriodicTimer::new(timg0.timer0);
    sensor_timer.set_interrupt_handler(sensor_timer_isr);
    sensor_timer.start(Duration::from_millis(5000)).unwrap();
    sensor_timer.listen();
    critical_section::with(|cs| {
        ISR_SENSOR_TIMER.borrow_ref_mut(cs).replace(sensor_timer);
    });

    let mut button_timer = OneShotTimer::new(timg1.timer0);
    button_timer.set_interrupt_handler(button_timer_isr);
    button_timer.listen();
    critical_section::with(|cs| {
        ISR_SCREEN_TIMEOUT.borrow_ref_mut(cs).replace(button_timer);
    });

    let i2c_config = i2c_master::Config::default();
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO5)
        .with_scl(peripherals.GPIO6);
    let i2c = RefCell::new(i2c);

    let mut sensor = Scd4x::new(RefCellDevice::new(&i2c), Delay::new());
    sensor.stop_periodic_measurement().unwrap();
    sensor.reinit().unwrap();
    sensor.set_automatic_self_calibration(true).unwrap();
    sensor.start_periodic_measurement().unwrap();

    let mut display = Display::new(RefCellDevice::new(&i2c));
    display.init();

    let mut measurement: Option<SensorData> = None;
    loop {
        // Wait for interruption
        unsafe { core::arch::asm!("wfi") }

        if get_and_clear_isr_flag(&IS_GPIO_ISR) {
            if let Some(measurement) = measurement.as_ref() {
                display.toggle_on_with(measurement);
                critical_section::with(|cs| {
                    let mut button = ISR_SCREEN_TIMEOUT.borrow_ref_mut(cs);
                    if let Some(button) = button.as_mut() {
                        button.schedule(Duration::from_millis(3000)).unwrap();
                    }
                });
            }
        }
        if get_and_clear_isr_flag(&IS_SENSOR_TIMER_ISR) {
            if sensor.data_ready_status().is_ok_and(|x| x) {
                measurement = Some(sensor.measurement().unwrap());
            }
        }
        if get_and_clear_isr_flag(&IS_SCREEN_TIMEOUT_ISR) {
            display.toggle_off();
        }
    }
}
