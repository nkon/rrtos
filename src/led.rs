use embedded_hal::digital::{OutputPin, StatefulOutputPin};
use rp2040_hal::gpio::{bank0::Gpio25, FunctionSio, Pin, PullDown, SioOutput};

use crate::mutex::Mutex;

struct Led {
    pin: Option<Pin<Gpio25, FunctionSio<SioOutput>, PullDown>>,
}

impl Led {
    const fn new() -> Led {
        Led { pin: None }
    }
}

static LED: Mutex<Led> = Mutex::new(Led::new());

pub fn init(pin: Pin<Gpio25, FunctionSio<SioOutput>, PullDown>) {
    LED.lock().pin = Some(pin);
}

pub fn set_output(pin: bool) {
    if pin {
        let _ = LED.lock().pin.as_mut().unwrap().set_high();
    } else {
        let _ = LED.lock().pin.as_mut().unwrap().set_low();
    }
}

pub fn toggle() {
    let _ = LED.lock().pin.as_mut().unwrap().toggle();
}
