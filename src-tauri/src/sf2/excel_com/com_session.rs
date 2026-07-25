use crate::domain::error::{AppError, Result};
use std::cell::Cell;
use std::path::Path;
use windows::core::{BSTR, GUID, PCWSTR};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::System::Com::{
    CLSIDFromProgID, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, DISPATCH_FLAGS, DISPATCH_METHOD,
    DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS,
};
use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
use windows::Win32::System::Variant::{
    VariantClear, VARENUM, VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_BSTR,
    VT_DISPATCH, VT_EMPTY, VT_I2, VT_I4, VT_I8, VT_INT, VT_NULL, VT_R4, VT_R8, VT_UI2, VT_UI4,
    VT_UI8, VT_UINT,
};

const LOCALE_USER_DEFAULT: u32 = 0x0400;

// ── COM Threading ────────────────────────────────────────────────────────────

pub(crate) fn run_excel_task<T, F>(task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    std::thread::spawn(move || {
        let _apartment = ComApartment::init()?;
        task()
    })
    .join()
    .map_err(|_| AppError::Internal("Excel automation thread panicked".to_string()))?
}

struct ComApartment;

impl ComApartment {
    fn init() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|error| {
                    AppError::Internal(format!("failed to initialize Excel automation: {error}"))
                })?;
        }
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

// ── Workbook Session Management ──────────────────────────────────────────────

pub(crate) fn with_workbook<T, F>(
    path: &Path,
    read_only: bool,
    save_on_close: bool,
    action: F,
) -> Result<T>
where
    F: FnOnce(&ExcelSession, &ComObject) -> Result<T>,
{
    let mut excel = ExcelSession::new()?;
    let workbook = excel.open_workbook(path, read_only)?;
    let action_result = action(&excel, &workbook);
    let close_result = workbook.method("Close", vec![ComVariant::bool(save_on_close)]);
    let quit_result = excel.quit();

    match (action_result, close_result, quit_result) {
        (Ok(value), Ok(_), Ok(_)) => Ok(value),
        (Err(error), _, _) => Err(error),
        (_, Err(error), _) => Err(error),
        (_, _, Err(error)) => Err(error),
    }
}

// ── ExcelSession ─────────────────────────────────────────────────────────────

pub(crate) struct ExcelSession {
    pub(crate) app: ComObject,
    quit_called: Cell<bool>,
}

impl ExcelSession {
    pub(crate) fn new() -> Result<Self> {
        let app = ComObject::excel_application()?;
        app.put_bool("Visible", false)?;
        app.put_bool("DisplayAlerts", false)?;
        app.put_bool("EnableEvents", false)?;
        let _ = app.put_i4("AutomationSecurity", 3);
        Ok(Self {
            app,
            quit_called: Cell::new(false),
        })
    }

    pub(crate) fn open_workbook(&self, path: &Path, read_only: bool) -> Result<ComObject> {
        let workbooks = self.app.get_object("Workbooks")?;
        workbooks
            .method_object(
                "Open",
                vec![
                    ComVariant::bstr(&path.to_string_lossy()),
                    ComVariant::i4(0),
                    ComVariant::bool(read_only),
                ],
            )
            .map_err(|original| {
                log::warn!("COM open_workbook failed: {original}");
                AppError::Internal(
                    "Could not open the SF2 workbook. \
                     If it is open in Microsoft Excel, close it first and try again."
                        .to_string(),
                )
            })
    }

    pub(crate) fn calculate_full_rebuild(&self) -> Result<()> {
        self.app.method("CalculateFullRebuild", Vec::new())?;
        Ok(())
    }

    pub(crate) fn quit(&mut self) -> Result<()> {
        if self.quit_called.replace(true) {
            return Ok(());
        }
        self.app.method("Quit", Vec::new())?;
        Ok(())
    }
}

impl Drop for ExcelSession {
    fn drop(&mut self) {
        let _ = self.quit();
    }
}

// ── ComObject ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ComObject {
    dispatch: IDispatch,
}

impl ComObject {
    fn excel_application() -> Result<Self> {
        let prog_id = wide_null("Excel.Application");
        let clsid = unsafe { CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) }.map_err(|error| {
            AppError::Internal(format!("Microsoft Excel is not available: {error}"))
        })?;
        let dispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER) }
            .map_err(|error| AppError::Internal(format!("failed to start Excel: {error}")))?;
        Ok(Self { dispatch })
    }

    pub fn get_object(&self, name: &str) -> Result<Self> {
        self.get(name)?.to_dispatch()
    }

    pub fn get_object_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)?.to_dispatch()
    }

    pub fn method_object(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.method(name, args)?.to_dispatch()
    }

    pub fn get_string(&self, name: &str) -> Result<String> {
        Ok(self.get(name)?.to_string_value())
    }

    pub fn get_i32(&self, name: &str) -> Result<i32> {
        self.get(name)?.to_i32()
    }

    pub fn get_bool(&self, name: &str) -> Result<bool> {
        self.get(name)?.to_bool()
    }

    pub fn put_bool(&self, name: &str, value: bool) -> Result<()> {
        self.put_variant(name, ComVariant::bool(value))
    }

    pub fn put_i4(&self, name: &str, value: i32) -> Result<()> {
        self.put_variant(name, ComVariant::i4(value))
    }

    pub fn put_string(&self, name: &str, value: &str) -> Result<()> {
        self.put_variant(name, ComVariant::bstr(value))
    }

    pub fn put_variant(&self, name: &str, value: ComVariant) -> Result<()> {
        self.invoke(name, DISPATCH_PROPERTYPUT, vec![value])?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())
    }

    pub fn get_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)
    }

    pub fn method(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_METHOD, args)
    }

    fn invoke(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        args: Vec<ComVariant>,
    ) -> Result<ComVariant> {
        let dispid = self.dispid(name)?;
        let mut raw_args = args
            .into_iter()
            .rev()
            .map(ComVariant::into_raw)
            .collect::<Vec<_>>();
        let mut property_put_dispid = DISPID_PROPERTYPUT;
        let is_property_put = flags == DISPATCH_PROPERTYPUT;
        let params = DISPPARAMS {
            rgvarg: if raw_args.is_empty() {
                std::ptr::null_mut()
            } else {
                raw_args.as_mut_ptr()
            },
            rgdispidNamedArgs: if is_property_put {
                &mut property_put_dispid
            } else {
                std::ptr::null_mut()
            },
            cArgs: raw_args.len() as u32,
            cNamedArgs: u32::from(is_property_put),
        };
        let mut result = VARIANT::default();
        let invoke_result = unsafe {
            self.dispatch.Invoke(
                dispid,
                &GUID::zeroed(),
                LOCALE_USER_DEFAULT,
                flags,
                &params,
                Some(&mut result),
                None,
                None,
            )
        };
        clear_variants(&mut raw_args);

        invoke_result.map_err(|error| {
            AppError::Internal(format!("Excel automation failed on {name}: {error}"))
        })?;
        Ok(ComVariant(result))
    }

    fn dispid(&self, name: &str) -> Result<i32> {
        let wide_name = wide_null(name);
        let names = [PCWSTR(wide_name.as_ptr())];
        let mut dispid = 0;
        unsafe {
            self.dispatch.GetIDsOfNames(
                &GUID::zeroed(),
                names.as_ptr(),
                names.len() as u32,
                LOCALE_USER_DEFAULT,
                &mut dispid,
            )
        }
        .map_err(|error| {
            AppError::Internal(format!(
                "Excel automation could not resolve {name}: {error}"
            ))
        })?;
        Ok(dispid)
    }
}

// ── ComVariant ───────────────────────────────────────────────────────────────

pub struct ComVariant(VARIANT);

impl ComVariant {
    pub fn empty() -> Self {
        Self(variant_from_type(VT_EMPTY, VARIANT_0_0_0 { lVal: 0 }))
    }

    pub fn i4(value: i32) -> Self {
        Self(variant_from_type(VT_I4, VARIANT_0_0_0 { lVal: value }))
    }

    pub fn bool(value: bool) -> Self {
        let value = if value {
            VARIANT_BOOL(-1)
        } else {
            VARIANT_BOOL(0)
        };
        Self(variant_from_type(VT_BOOL, VARIANT_0_0_0 { boolVal: value }))
    }

    pub fn bstr(value: &str) -> Self {
        Self(variant_from_type(
            VT_BSTR,
            VARIANT_0_0_0 {
                bstrVal: std::mem::ManuallyDrop::new(BSTR::from(value)),
            },
        ))
    }

    pub fn to_dispatch(&self) -> Result<ComObject> {
        if self.variant_type() != VT_DISPATCH {
            return Err(AppError::Internal(format!(
                "Excel automation returned {}, expected dispatch object",
                self.variant_type_name()
            )));
        }

        let dispatch = unsafe {
            let dispatch = &self.0.Anonymous.Anonymous.Anonymous.pdispVal;
            std::mem::ManuallyDrop::into_inner(dispatch.clone())
        }
        .ok_or_else(|| AppError::Internal("Excel returned a null object".to_string()))?;

        Ok(ComObject { dispatch })
    }

    pub fn to_string_value(&self) -> String {
        match self.variant_type() {
            VT_BSTR => unsafe { self.0.Anonymous.Anonymous.Anonymous.bstrVal.to_string() },
            VT_I4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal.to_string() },
            VT_I2 => unsafe { self.0.Anonymous.Anonymous.Anonymous.iVal.to_string() },
            VT_I8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.llVal.to_string() },
            VT_INT => unsafe { self.0.Anonymous.Anonymous.Anonymous.intVal.to_string() },
            VT_UI2 => unsafe { self.0.Anonymous.Anonymous.Anonymous.uiVal.to_string() },
            VT_UI4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.ulVal.to_string() },
            VT_UI8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.ullVal.to_string() },
            VT_UINT => unsafe { self.0.Anonymous.Anonymous.Anonymous.uintVal.to_string() },
            VT_R4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.fltVal.to_string() },
            VT_R8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.dblVal.to_string() },
            VT_BOOL => {
                if self.to_bool().unwrap_or(false) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            VT_EMPTY | VT_NULL => String::new(),
            _ => String::new(),
        }
    }

    pub fn to_i32(&self) -> Result<i32> {
        match self.variant_type() {
            VT_I4 => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal }),
            VT_I2 => Ok(i32::from(unsafe {
                self.0.Anonymous.Anonymous.Anonymous.iVal
            })),
            VT_I8 => i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.llVal })
                .map_err(|_| self.integer_range_error()),
            VT_INT => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.intVal }),
            VT_UI2 => Ok(i32::from(unsafe {
                self.0.Anonymous.Anonymous.Anonymous.uiVal
            })),
            VT_UI4 | VT_UINT => {
                i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.ulVal })
                    .map_err(|_| self.integer_range_error())
            }
            VT_UI8 => i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.ullVal })
                .map_err(|_| self.integer_range_error()),
            VT_R4 => {
                float_to_i32(unsafe { f64::from(self.0.Anonymous.Anonymous.Anonymous.fltVal) })
                    .map_err(|_| self.integer_range_error())
            }
            VT_R8 => float_to_i32(unsafe { self.0.Anonymous.Anonymous.Anonymous.dblVal })
                .map_err(|_| self.integer_range_error()),
            VT_BOOL => Ok(i32::from(self.to_bool()?)),
            _ => Err(AppError::Internal(format!(
                "Excel automation returned {}, expected integer",
                self.variant_type_name()
            ))),
        }
    }

    pub fn to_bool(&self) -> Result<bool> {
        match self.variant_type() {
            VT_BOOL => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.boolVal }.0 != 0),
            VT_I4 => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal } != 0),
            VT_EMPTY | VT_NULL => Ok(false),
            _ => Err(AppError::Internal(format!(
                "Excel automation returned {}, expected boolean",
                self.variant_type_name()
            ))),
        }
    }

    fn variant_type(&self) -> VARENUM {
        unsafe { self.0.Anonymous.Anonymous.vt }
    }

    fn variant_type_name(&self) -> String {
        format!("VARIANT({})", self.variant_type().0)
    }

    fn integer_range_error(&self) -> AppError {
        AppError::Internal(format!(
            "Excel automation returned {}, but it is not a valid integer",
            self.variant_type_name()
        ))
    }

    fn into_raw(mut self) -> VARIANT {
        std::mem::take(&mut self.0)
    }
}

impl Drop for ComVariant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

fn variant_from_type(vt: VARENUM, value: VARIANT_0_0_0) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 {
                vt,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: value,
            }),
        },
    }
}

fn clear_variants(variants: &mut [VARIANT]) {
    for variant in variants {
        unsafe {
            let _ = VariantClear(variant);
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn float_to_i32(value: f64) -> std::result::Result<i32, ()> {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= f64::from(i32::MIN)
        && value <= f64::from(i32::MAX)
    {
        Ok(value as i32)
    } else {
        Err(())
    }
}
