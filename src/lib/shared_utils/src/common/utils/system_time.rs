use ic_cdk::api;
use std::time::SystemTime;
use std::time::{Duration, UNIX_EPOCH};

pub type SystemTimeProvider = dyn Fn() -> SystemTime;

pub fn get_current_system_time_from_ic() -> SystemTime {
    UNIX_EPOCH
        .checked_add(Duration::new(
            api::time() / 1_000_000_000,
            (api::time() % 1_000_000_000) as u32,
        ))
        .expect("Getting timestamp from ic_cdk failed")
}

#[cfg(not(feature = "mockdata"))]
pub fn get_current_system_time() -> SystemTime {
    get_current_system_time_from_ic()
}

#[cfg(feature = "mockdata")]
pub mod mock_time {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static MOCK_TIME: RefCell<Option<SystemTime>> = RefCell::new(None);
    }

    pub fn get_current_system_time() -> SystemTime {
        MOCK_TIME.with(|cell| {
            cell.borrow()
                .as_ref()
                .cloned()
                .unwrap_or_else(SystemTime::now)
        })
    }

    pub fn set_mock_time(time: SystemTime) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = Some(time));
    }

    pub fn clear_mock_time() {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = None);
    }
}

#[cfg(feature = "mockdata")]
pub use mock_time::get_current_system_time;
#[cfg(feature = "mockdata")]
pub use mock_time::set_mock_time;
