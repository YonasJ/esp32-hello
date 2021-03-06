#![feature(never_type)]
#![no_std]

use core::fmt::Write;

#[macro_use]
extern crate alloc;

use macaddr::MacAddr6;

pub mod gpio;
pub mod ets;
pub mod uart;
pub mod wifi;
pub mod nvs;

#[cfg(feature = "panic_handler")]
#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
  hprintln!("{}", panic_info);

  unsafe {
    abort();
    core::hint::unreachable_unchecked()
  }
}

#[macro_export]
macro_rules! hprint {
  ($($s:expr),*) => {{
    use core::fmt::Write;
    write!(crate::ets::Ets, $($s),*).unwrap();
  }}
}

#[macro_export]
macro_rules! hprintln {
  ($($s:expr),*) => {{
    use core::fmt::Write;
    writeln!(crate::ets::Ets, $($s),*).unwrap();
  }}
}

#[macro_export]
macro_rules! cstring {
  ($s:expr) => {{
    let mut name: Vec<libc::c_char> = $s.chars().map(|c| c as libc::c_char).collect();
    name.push(0);
    name
  }}
}


#[macro_export]
macro_rules! ptr_set_mask {
  ($register:expr, $mask:expr) => {
    let ptr = $register as *mut u32;
    core::ptr::write_volatile(ptr, core::ptr::read_volatile(ptr) | ($mask));
  };
}

#[macro_export]
macro_rules! ptr_clear_mask {
  ($register:expr, $mask:expr) => {
    let ptr = $register as *mut u32;
    core::ptr::write_volatile(ptr, core::ptr::read_volatile(ptr) & !($mask));
  };
}

use esp_idf_sys::esp_err_t;

#[derive(Clone, Debug)]
pub struct EspError { code: esp_err_t }

impl EspError {
  pub fn result(code: esp_err_t) -> Result<(), Self> {
    use esp_idf_sys::ESP_OK;

    if code == ESP_OK as esp_err_t {
      return Ok(())
    } else {
      Err(EspError { code })
    }
  }
}

impl From<!> for EspError {
  fn from(never: !) -> Self {
    loop {}
  }
}

impl From<esp_err_t> for EspError {
  fn from(code: esp_err_t) -> Self {
    EspError { code }
  }
}

impl core::fmt::Display for EspError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    use esp_idf_sys::esp_err_to_name;

    unsafe {
      let mut name = esp_err_to_name(self.code);

      while !name.is_null() {
        let c = core::char::from_u32_unchecked(*name as u32);

        if c == '\0' { break }
        f.write_char(c)?;
        name = unsafe { name.add(1) };
      }
    }

    Ok(())
  }
}

#[repr(C)]
pub enum MacAddressType {
  Sta,
  Ap,
  #[cfg(not(target_device = "esp8266"))]
  Bt,
  #[cfg(not(target_device = "esp8266"))]
  Eth,
}

pub fn mac_address(t: MacAddressType) -> MacAddr6 {
  use esp_idf_sys::{esp_read_mac, esp_mac_type_t};

  let t = match t {
    MacAddressType::Sta => esp_idf_sys::esp_mac_type_t::ESP_MAC_WIFI_STA,
    MacAddressType::Ap  => esp_idf_sys::esp_mac_type_t::ESP_MAC_WIFI_SOFTAP,
    #[cfg(not(target_device = "esp8266"))]
    MacAddressType::Bt  => esp_idf_sys::esp_mac_type_t::ESP_MAC_BT,
    #[cfg(not(target_device = "esp8266"))]
    MacAddressType::Eth => esp_idf_sys::esp_mac_type_t::ESP_MAC_ETH,
  };

  let mut mac_address = MacAddr6::nil();
  unsafe { esp_read_mac(&mut mac_address as *mut _ as *mut _, t) };
  mac_address
}
