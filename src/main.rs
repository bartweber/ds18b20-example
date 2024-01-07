#![no_main]
#![no_std]

use cortex_m::iprintln;
use cortex_m::peripheral::ITM;
use panic_itm; // panic handler

use cortex_m_rt::entry;
use ds18b20::{Ds18b20, Resolution};

use stm32f3_discovery::stm32f3xx_hal;

use stm32f3xx_hal::prelude::*;
use stm32f3xx_hal::{
    delay::Delay,
    pac,
};

use one_wire_bus::OneWire;

#[entry]
fn main() -> ! {
    let device_periphs = pac::Peripherals::take().unwrap();
    let core_periphs = cortex_m::Peripherals::take().unwrap();

    let mut itm: ITM = core_periphs.ITM;

    let mut reset_and_clock_control = device_periphs.RCC.constrain();

    let mut flash = device_periphs.FLASH.constrain();
    let clocks = reset_and_clock_control.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(core_periphs.SYST, clocks);

    let mut gpioa = device_periphs.GPIOA.split(&mut reset_and_clock_control.ahb);

    let one_wire_pin = gpioa.pa1.into_open_drain_output(&mut gpioa.moder, &mut gpioa.otyper);

    let mut one_wire_bus = OneWire::new(one_wire_pin).unwrap();

    loop {
        // initiate a temperature measurement for all connected devices
        ds18b20::start_simultaneous_temp_measurement(&mut one_wire_bus, &mut delay).unwrap();

        // wait until the measurement is done. This depends on the resolution you specified
        // If you don't know the resolution, you can obtain it from reading the sensor data,
        // or just wait the longest time, which is the 12-bit resolution (750ms)
        Resolution::Bits12.delay_for_measurement_time(&mut delay);

        // iterate over all the devices, and report their temperature
        let mut search_state = None;
        loop {
            let result = one_wire_bus.device_search(search_state.as_ref(), false, &mut delay);
            if let Err(err) = result {
                iprintln!(&mut itm.stim[0], "Error searching for devices: {:?}", err);
                break;
            }
            if let Some((device_address, state)) = result.unwrap() {
                search_state = Some(state);
                if device_address.family_code() != ds18b20::FAMILY_CODE {
                    // skip other devices
                    continue;
                }
                // You will generally create the sensor once, and save it for later
                let sensor: Ds18b20 = Ds18b20::new::<Infallible>(device_address).unwrap();

                // contains the read temperature, as well as config info such as the resolution used
                let sensor_data = sensor.read_data(&mut one_wire_bus, &mut delay).unwrap();
                iprintln!(&mut itm.stim[0], "Device at {:?} is {}°C", device_address, sensor_data.temperature);
                // writeln!(tx, "Device at {:?} is {}°C", device_address, sensor_data.temperature);
            } else {
                break;
            }
        }

        delay.delay_ms(2000_u16);
    }
}
