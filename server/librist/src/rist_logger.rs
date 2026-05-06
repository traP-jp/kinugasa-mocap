use crate::*;

/// A builder for creating `RistLogger` instances.
///
/// TODO: `log_stream` and `log_socket` mode is not supported for now.
#[derive(Default)]
pub enum RistLoggerBuilder<CBLogger = RistStderrCallBackLogger>
where
    CBLogger: RistCallBackLogger,
{
    CallBack(std::sync::Arc<CBLogger>),
    #[default]
    NoLogging,
}

impl<CBLogger> Clone for RistLoggerBuilder<CBLogger>
where
    CBLogger: RistCallBackLogger,
{
    fn clone(&self) -> Self {
        match self {
            RistLoggerBuilder::CallBack(logger) => RistLoggerBuilder::CallBack(logger.clone()),
            RistLoggerBuilder::NoLogging => RistLoggerBuilder::NoLogging,
        }
    }
}

impl<CBLogger> RistLoggerBuilder<CBLogger>
where
    CBLogger: RistCallBackLogger,
{
    pub(crate) fn initialize(&self) -> RistLogger<CBLogger> {
        match self {
            RistLoggerBuilder::CallBack(logger) => RistLogger {
                log_cb_arg: Some(std::sync::Arc::into_raw(logger.clone()) as *const CBLogger),
                _phantom: std::marker::PhantomData,
            },
            RistLoggerBuilder::NoLogging => RistLogger {
                log_cb_arg: None,
                _phantom: std::marker::PhantomData,
            },
        }
    }

    /// Returns `log_cb` function.
    pub(crate) fn log_cb(
        &self,
    ) -> Option<
        unsafe extern "C" fn(
            *mut std::os::raw::c_void,
            crate::binding::rist_log_level,
            *const ::std::os::raw::c_char,
        ) -> i32,
    > {
        match self {
            RistLoggerBuilder::CallBack(_) => Some(CBLogger::call_raw),
            RistLoggerBuilder::NoLogging => None,
        }
    }
}

/// `RistLogger` is a wrapper for managing logging functionality.
///
/// ### Safety(Internal)
/// Drop of this instance means that logger is no longer available.
pub struct RistLogger<CBLogger = RistStderrCallBackLogger>
where
    CBLogger: RistCallBackLogger,
{
    log_cb_arg: Option<*const CBLogger>,
    _phantom: std::marker::PhantomData<CBLogger>,
}

impl<CBLogger> Drop for RistLogger<CBLogger>
where
    CBLogger: RistCallBackLogger,
{
    fn drop(&mut self) {
        if let Some(ptr) = self.log_cb_arg {
            unsafe {
                std::sync::Arc::from_raw(ptr as *const CBLogger);
            }
        }
    }
}

impl<CBLogger> RistLogger<CBLogger>
where
    CBLogger: RistCallBackLogger,
{
    /// Returns a mutable pointer to the log callback argument.
    ///
    /// # Safety
    /// The returned pointer must not be used after the `RistLogger` instance is dropped.
    pub(crate) unsafe fn as_log_cb_arg(&self) -> *mut std::os::raw::c_void {
        self.log_cb_arg.unwrap_or(std::ptr::null_mut()) as *mut std::os::raw::c_void
    }

    pub(crate) fn log_socket(&self) -> i32 {
        -1
    }

    pub(crate) fn log_stream(&self) -> *mut crate::binding::_IO_FILE {
        std::ptr::null_mut()
    }
}

/// When user-defined callback returns `RistCallBackLoggerUserCBFailedError`,
/// callback of the internal C library will be returned non-zero integer.
#[derive(Debug, Clone)]
pub struct RistCallBackLoggerUserCBFailedError {}

/// `RistCallBackLogger` represents both `log_cb` and `log_cb_arg`.
pub trait RistCallBackLogger: Send + Sync {
    /// `call` will be called when the C library needs to log a message
    /// and a pair of log level and message is available properly.
    fn call(
        &self,
        log_level: &crate::RistLogLevel,
        msg: &str,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError>;

    /// Called when the wrapper library detects some malformed behaviour of the C library.
    fn call_malformed(
        &self,
        error: RistCallBackLoggerError,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        self.call(
            &crate::RistLogLevel::Error,
            &format!(
                "rust-wrapper: RistCallBackLogger::call_malformed: {:?}",
                error
            ),
        )?;
        Err(RistCallBackLoggerUserCBFailedError {})
    }

    /// Called when the wrapper library failed to acquire the pointer of `RistCallBackLogger` instance.
    fn call_malformed_global(
        error: RistCallBackLoggerError,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        println!(
            "rust-wrapper: RistCallBackLogger::call_malformed_global: {:?}",
            error
        );
        Err(RistCallBackLoggerUserCBFailedError {})
    }
}

#[derive(Debug, Clone)]
pub enum RistCallBackLoggerError {
    UnknownEnumVariant(crate::RistUnknownEnumVariantError),
    InvalidPointer(crate::RistInvalidPointerError),
    Utf8Error(std::str::Utf8Error),
}

pub(crate) trait RistCallBackLoggerInternal: RistCallBackLogger {
    unsafe extern "C" fn call_raw(
        arg: *mut std::os::raw::c_void,
        log_level: crate::binding::rist_log_level,
        msg: *const ::std::os::raw::c_char,
    ) -> i32
    where
        Self: Sized,
    {
        if arg.is_null() {
            return Self::panic_free_call_malformed_global(RistCallBackLoggerError::InvalidPointer(
                crate::RistInvalidPointerError {
                    message: "Invalid pointer for `arg` in log_cb.",
                },
            ))
            .is_err() as i32;
        }
        let logger = unsafe { &*(arg as *const Self) };
        let log_level: crate::RistLogLevel = match log_level.try_into() {
            Ok(level) => level,
            Err(error) => {
                return logger
                    .panic_free_call_malformed(RistCallBackLoggerError::UnknownEnumVariant(error))
                    .is_err() as i32;
            }
        };
        if msg.is_null() {
            return logger
                .panic_free_call_malformed(RistCallBackLoggerError::InvalidPointer(
                    crate::RistInvalidPointerError {
                        message: "Invalid pointer for `msg` in log_cb.",
                    },
                ))
                .is_err() as i32;
        }
        let msg: &str = match unsafe { std::ffi::CStr::from_ptr(msg).to_str() } {
            Ok(s) => s,
            Err(error) => {
                return logger
                    .panic_free_call_malformed(RistCallBackLoggerError::Utf8Error(error))
                    .is_err() as i32;
            }
        };
        logger.panic_free_call(&log_level, msg).is_err() as i32
    }

    fn panic_free_call(
        &self,
        log_level: &crate::RistLogLevel,
        msg: &str,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.call(log_level, msg)))
            .map_err(|_| RistCallBackLoggerUserCBFailedError {})
            .flatten()
    }

    fn panic_free_call_malformed(
        &self,
        error: RistCallBackLoggerError,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.call_malformed(error)))
            .map_err(|_| RistCallBackLoggerUserCBFailedError {})
            .flatten()
    }

    fn panic_free_call_malformed_global(
        error: RistCallBackLoggerError,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Self::call_malformed_global(error)
        }))
        .map_err(|_| RistCallBackLoggerUserCBFailedError {})
        .flatten()
    }
}

impl<T: RistCallBackLogger> RistCallBackLoggerInternal for T {}

pub struct RistStderrCallBackLogger {
    log_level: RistLogLevel,
}

impl RistCallBackLogger for RistStderrCallBackLogger {
    fn call(
        &self,
        log_level: &crate::RistLogLevel,
        msg: &str,
    ) -> Result<(), RistCallBackLoggerUserCBFailedError> {
        if self.log_level.is_important_than_or_equal_to(log_level) {
            eprintln!("[{}] {}", log_level, msg);
        }
        Ok(())
    }
}
