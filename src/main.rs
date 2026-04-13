#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod atomic_bool;
mod block_average;
mod display;
mod ring_buffer;

use core::cell::RefCell;

use crate::atomic_bool::MyAtomicBool;
use crate::block_average::BlockAverage;
use crate::ring_buffer::RingBuffer;
use critical_section::Mutex;
use display::Display;
use embedded_hal_bus::i2c::RefCellDevice;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Event, Input, InputConfig, Io, Pull};
use esp_hal::i2c::master::{self as i2c_master, I2c};
use esp_hal::time::Duration;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::timer::{OneShotTimer, PeriodicTimer};
use esp_hal::{handler, main, Blocking};
use esp_println::println;
use scd4x::Scd4x;

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("{}", e);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

static ISR_INPUT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static ISR_SENSOR_TIMER: Mutex<RefCell<Option<PeriodicTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));
static ISR_APP_TIMER: Mutex<RefCell<Option<OneShotTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));

// use custom AtomicBool to do things in critical_section
static IS_BUTTON_ISR: MyAtomicBool = MyAtomicBool::new(false);
static IS_SENSOR_TIMER_ISR: MyAtomicBool = MyAtomicBool::new(false);
static IS_APP_TIMER_ISR: MyAtomicBool = MyAtomicBool::new(false);

const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(300);
const SCREEN_TIMEOUT: Duration = Duration::from_millis(5000);

fn schedule_app_timer(duration: Duration) {
    critical_section::with(|cs| {
        let mut timer = ISR_APP_TIMER.borrow_ref_mut(cs);
        if let Some(timer) = timer.as_mut() {
            timer.stop();
            timer.schedule(duration).unwrap();
        }
    });
}

#[handler]
fn gpio_isr() {
    critical_section::with(|cs| {
        let mut button = ISR_INPUT.borrow_ref_mut(cs);
        let Some(button) = button.as_mut() else {
            return;
        };
        if !button.is_interrupt_set() {
            button.clear_interrupt();
            return;
        }
        IS_BUTTON_ISR.store(true, cs);
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
        IS_SENSOR_TIMER_ISR.store(true, cs);
        timer.clear_interrupt();
    });
}

#[handler]
fn button_timer_isr() {
    critical_section::with(|cs| {
        let mut timer = ISR_APP_TIMER.borrow_ref_mut(cs);
        let Some(timer) = timer.as_mut() else {
            return;
        };
        IS_APP_TIMER_ISR.store(true, cs);
        timer.clear_interrupt();
    });
}

enum AppState {
    Initializing,
    Idle,
    GeneralViewDebouncing,
    GeneralView,
    HistoryViewDebouncing,
    HistoryView,
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
        ISR_APP_TIMER.borrow_ref_mut(cs).replace(button_timer);
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
    display.toggle_on_with_initialization_message();

    let mut app_state = AppState::Initializing;

    let mut co2_history = RingBuffer::<u16, 120>::new();
    let mut co2_average = BlockAverage::new(12);
    let mut measurement = None;

    loop {
        // Wait for interruption
        unsafe { core::arch::asm!("wfi") }

        if IS_BUTTON_ISR.swap_in_cs(false) {
            app_state = match app_state {
                AppState::Initializing => AppState::Initializing,
                AppState::Idle | AppState::HistoryView => {
                    if let Some(measurement) = measurement.as_ref() {
                        display.toggle_on_with_measurement(measurement);
                    }
                    schedule_app_timer(DEBOUNCE_TIMEOUT);
                    AppState::GeneralViewDebouncing
                }
                AppState::GeneralView => {
                    display.toggle_on_with_history(&co2_history);
                    schedule_app_timer(DEBOUNCE_TIMEOUT);
                    AppState::HistoryViewDebouncing
                }
                AppState::GeneralViewDebouncing | AppState::HistoryViewDebouncing => {
                    // reset debounce timer
                    schedule_app_timer(DEBOUNCE_TIMEOUT);
                    app_state
                }
            };
        }

        if IS_SENSOR_TIMER_ISR.swap_in_cs(false) {
            if sensor.data_ready_status().is_ok_and(|x| x) {
                if measurement.is_none() {
                    app_state = AppState::Idle;
                    display.toggle_off();
                }
                let new_measurement = sensor.measurement().unwrap();

                if let Some(average) = co2_average.push(new_measurement.co2) {
                    co2_history.push(average);
                }

                measurement = Some(new_measurement);
            }
        }

        if IS_APP_TIMER_ISR.swap_in_cs(false) {
            app_state = match app_state {
                AppState::Initializing => AppState::Initializing,
                AppState::Idle => AppState::Idle,
                AppState::GeneralViewDebouncing => {
                    schedule_app_timer(SCREEN_TIMEOUT);
                    AppState::GeneralView
                }
                AppState::HistoryViewDebouncing => {
                    schedule_app_timer(SCREEN_TIMEOUT);
                    AppState::HistoryView
                }
                AppState::GeneralView | AppState::HistoryView => {
                    display.toggle_off();
                    AppState::Idle
                }
            }
        }
    }
}
