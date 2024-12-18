use embedded_hal::digital::{OutputPin, StatefulOutputPin};
use rp2040_hal::gpio::{bank0::Gpio25, FunctionSio, Pin, PullDown, SioOutput};

use crate::mutex::Mutex;

static LED: Mutex<Option<Pin<Gpio25, FunctionSio<SioOutput>, PullDown>>> = Mutex::new(None);

pub fn init(pin: Pin<Gpio25, FunctionSio<SioOutput>, PullDown>) {
    LED.lock().replace(pin);
}

pub fn set_output(pin: bool) {
    if pin {
        let _ = LED.lock().as_mut().unwrap().set_high();
    } else {
        let _ = LED.lock().as_mut().unwrap().set_low();
    }
}

pub fn toggle() {
    let _ = LED.lock().as_mut().unwrap().toggle();
}
