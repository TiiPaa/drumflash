use std::{
    ffi::c_void,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr::{copy_nonoverlapping, null, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

use windows_sys::{
    core::{GUID, HRESULT, IID_IUnknown, IUnknown_Vtbl},
    Win32::{
        Foundation::{
            DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC,
            E_NOINTERFACE, E_NOTIMPL, OLE_E_ADVISENOTSUPPORTED, RPC_E_CHANGED_MODE, S_FALSE,
            S_OK,
        },
        System::{
            Com::{FORMATETC, STGMEDIUM, STGMEDIUM_0, DATADIR_GET, DVASPECT_CONTENT, TYMED_HGLOBAL},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
            Ole::{
                DoDragDrop, OleInitialize, OleUninitialize, CF_HDROP, DROPEFFECT_COPY,
                DROPEFFECT_NONE,
            },
        },
        UI::Shell::DROPFILES,
    },
};

const IID_IDATAOBJECT: GUID = GUID::from_u128(0x0000010e_0000_0000_c000_000000000046);
const IID_IDROPSOURCE: GUID = GUID::from_u128(0x00000121_0000_0000_c000_000000000046);
const MK_LBUTTON: u32 = 0x0001;

#[link(name = "kernel32")]
extern "system" {
    fn GlobalFree(hmem: *mut c_void) -> *mut c_void;
}

#[repr(C)]
struct DataObjectVtbl {
    base: IUnknown_Vtbl,
    get_data:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    get_data_here:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    query_get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC) -> HRESULT,
    get_canonical_format_etc:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut FORMATETC) -> HRESULT,
    set_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *const STGMEDIUM, i32) -> HRESULT,
    enum_format_etc: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT,
    d_advise:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, u32, *mut c_void, *mut u32) -> HRESULT,
    d_unadvise: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
    enum_d_advise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct DropSourceVtbl {
    base: IUnknown_Vtbl,
    query_continue_drag: unsafe extern "system" fn(*mut c_void, i32, u32) -> HRESULT,
    give_feedback: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
}

#[repr(C)]
struct MidiFileDataObject {
    vtbl: *const DataObjectVtbl,
    ref_count: AtomicU32,
    path_wide: Vec<u16>,
}

#[repr(C)]
struct MidiFileDropSource {
    vtbl: *const DropSourceVtbl,
    ref_count: AtomicU32,
}

static DATA_OBJECT_VTBL: DataObjectVtbl = DataObjectVtbl {
    base: IUnknown_Vtbl {
        QueryInterface: data_query_interface,
        AddRef: data_add_ref,
        Release: data_release,
    },
    get_data: data_get_data,
    get_data_here: data_get_data_here,
    query_get_data: data_query_get_data,
    get_canonical_format_etc: data_get_canonical_format_etc,
    set_data: data_set_data,
    enum_format_etc: data_enum_format_etc,
    d_advise: data_d_advise,
    d_unadvise: data_d_unadvise,
    enum_d_advise: data_enum_d_advise,
};

static DROP_SOURCE_VTBL: DropSourceVtbl = DropSourceVtbl {
    base: IUnknown_Vtbl {
        QueryInterface: source_query_interface,
        AddRef: source_add_ref,
        Release: source_release,
    },
    query_continue_drag: source_query_continue_drag,
    give_feedback: source_give_feedback,
};

pub fn start_midi_file_drag(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("MIDI file does not exist".to_string());
    }

    let mut path_wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if path_wide.is_empty() {
        return Err("MIDI path is empty".to_string());
    }
    path_wide.push(0);

    unsafe {
        let init_hr = OleInitialize(null());
        let initialized = init_hr == S_OK || init_hr == S_FALSE;
        if !initialized && init_hr != RPC_E_CHANGED_MODE {
            return Err(format!("OleInitialize failed: 0x{:08X}", init_hr as u32));
        }
        if init_hr == RPC_E_CHANGED_MODE {
            return Err("OLE drag-and-drop unavailable on this host UI thread".to_string());
        }

        let mut data_object = MidiFileDataObject {
            vtbl: &DATA_OBJECT_VTBL,
            ref_count: AtomicU32::new(1),
            path_wide,
        };
        let mut drop_source = MidiFileDropSource {
            vtbl: &DROP_SOURCE_VTBL,
            ref_count: AtomicU32::new(1),
        };
        let mut effect = DROPEFFECT_NONE;

        let hr = DoDragDrop(
            &mut data_object as *mut _ as *mut c_void,
            &mut drop_source as *mut _ as *mut c_void,
            DROPEFFECT_COPY,
            &mut effect,
        );

        OleUninitialize();

        if hr >= 0 || hr == DRAGDROP_S_CANCEL {
            Ok(())
        } else {
            Err(format!("DoDragDrop failed: 0x{:08X}", hr as u32))
        }
    }
}

unsafe extern "system" fn data_query_interface(
    this: *mut c_void,
    iid: *const GUID,
    interface: *mut *mut c_void,
) -> HRESULT {
    if interface.is_null() {
        return E_NOINTERFACE;
    }

    if guid_eq(iid, &IID_IUnknown) || guid_eq(iid, &IID_IDATAOBJECT) {
        *interface = this;
        data_add_ref(this);
        S_OK
    } else {
        *interface = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn data_add_ref(this: *mut c_void) -> u32 {
    let object = this as *mut MidiFileDataObject;
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn data_release(this: *mut c_void) -> u32 {
    let object = this as *mut MidiFileDataObject;
    (*object).ref_count.fetch_sub(1, Ordering::Release) - 1
}

unsafe extern "system" fn data_get_data(
    this: *mut c_void,
    format: *const FORMATETC,
    medium: *mut STGMEDIUM,
) -> HRESULT {
    if medium.is_null() {
        return DV_E_FORMATETC;
    }
    let query = data_query_get_data(this, format);
    if query != S_OK {
        return query;
    }

    let object = &*(this as *const MidiFileDataObject);
    match build_hdrop_medium(&object.path_wide) {
        Ok(stgmedium) => {
            *medium = stgmedium;
            S_OK
        }
        Err(hr) => hr,
    }
}

unsafe extern "system" fn data_get_data_here(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _medium: *mut STGMEDIUM,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_query_get_data(
    _this: *mut c_void,
    format: *const FORMATETC,
) -> HRESULT {
    if format.is_null() {
        return DV_E_FORMATETC;
    }

    let format = &*format;
    let has_hdrop = format.cfFormat == CF_HDROP;
    let has_content = format.dwAspect & DVASPECT_CONTENT != 0;
    let has_global = format.tymed & TYMED_HGLOBAL as u32 != 0;

    if has_hdrop && has_content && has_global {
        S_OK
    } else {
        DV_E_FORMATETC
    }
}

unsafe extern "system" fn data_get_canonical_format_etc(
    _this: *mut c_void,
    _format_in: *const FORMATETC,
    format_out: *mut FORMATETC,
) -> HRESULT {
    if !format_out.is_null() {
        (*format_out).ptd = null_mut();
    }
    DV_E_FORMATETC
}

unsafe extern "system" fn data_set_data(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _medium: *const STGMEDIUM,
    _release: i32,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_enum_format_etc(
    _this: *mut c_void,
    direction: u32,
    enum_format: *mut *mut c_void,
) -> HRESULT {
    if !enum_format.is_null() {
        *enum_format = null_mut();
    }
    if direction == DATADIR_GET as u32 {
        E_NOTIMPL
    } else {
        DV_E_FORMATETC
    }
}

unsafe extern "system" fn data_d_advise(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _advf: u32,
    _sink: *mut c_void,
    connection: *mut u32,
) -> HRESULT {
    if !connection.is_null() {
        *connection = 0;
    }
    OLE_E_ADVISENOTSUPPORTED
}

unsafe extern "system" fn data_d_unadvise(_this: *mut c_void, _connection: u32) -> HRESULT {
    OLE_E_ADVISENOTSUPPORTED
}

unsafe extern "system" fn data_enum_d_advise(
    _this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> HRESULT {
    if !enum_advise.is_null() {
        *enum_advise = null_mut();
    }
    OLE_E_ADVISENOTSUPPORTED
}

unsafe extern "system" fn source_query_interface(
    this: *mut c_void,
    iid: *const GUID,
    interface: *mut *mut c_void,
) -> HRESULT {
    if interface.is_null() {
        return E_NOINTERFACE;
    }

    if guid_eq(iid, &IID_IUnknown) || guid_eq(iid, &IID_IDROPSOURCE) {
        *interface = this;
        source_add_ref(this);
        S_OK
    } else {
        *interface = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn source_add_ref(this: *mut c_void) -> u32 {
    let object = this as *mut MidiFileDropSource;
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn source_release(this: *mut c_void) -> u32 {
    let object = this as *mut MidiFileDropSource;
    (*object).ref_count.fetch_sub(1, Ordering::Release) - 1
}

unsafe extern "system" fn source_query_continue_drag(
    _this: *mut c_void,
    escape_pressed: i32,
    key_state: u32,
) -> HRESULT {
    if escape_pressed != 0 {
        DRAGDROP_S_CANCEL
    } else if key_state & MK_LBUTTON == 0 {
        DRAGDROP_S_DROP
    } else {
        S_OK
    }
}

unsafe extern "system" fn source_give_feedback(_this: *mut c_void, _effect: u32) -> HRESULT {
    DRAGDROP_S_USEDEFAULTCURSORS
}

unsafe fn build_hdrop_medium(path_wide: &[u16]) -> Result<STGMEDIUM, HRESULT> {
    let path_bytes = (path_wide.len() + 1) * size_of::<u16>();
    let dropfiles_size = size_of::<DROPFILES>();
    let total_size = dropfiles_size + path_bytes;

    let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size);
    if hglobal.is_null() {
        return Err(DV_E_FORMATETC);
    }

    let memory = GlobalLock(hglobal);
    if memory.is_null() {
        GlobalFree(hglobal);
        return Err(DV_E_FORMATETC);
    }

    let dropfiles = memory as *mut DROPFILES;
    (*dropfiles).pFiles = dropfiles_size as u32;
    (*dropfiles).fWide = 1;

    let file_list = (memory as *mut u8).add(dropfiles_size) as *mut u16;
    copy_nonoverlapping(path_wide.as_ptr(), file_list, path_wide.len());
    *file_list.add(path_wide.len()) = 0;

    GlobalUnlock(hglobal);

    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL as u32,
        u: STGMEDIUM_0 { hGlobal: hglobal },
        pUnkForRelease: null_mut(),
    })
}

unsafe fn guid_eq(iid: *const GUID, expected: &GUID) -> bool {
    if iid.is_null() {
        return false;
    }
    let iid = &*iid;
    iid.data1 == expected.data1
        && iid.data2 == expected.data2
        && iid.data3 == expected.data3
        && iid.data4 == expected.data4
}
