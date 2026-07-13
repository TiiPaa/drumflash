use std::{
    ffi::c_void,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr::{copy_nonoverlapping, null, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

use windows_sys::{
    core::{IID_IUnknown, IUnknown_Vtbl, GUID, HRESULT},
    Win32::{
        Foundation::{
            DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC,
            E_NOINTERFACE, E_NOTIMPL, OLE_E_ADVISENOTSUPPORTED, RPC_E_CHANGED_MODE, S_FALSE, S_OK,
        },
        System::{
            Com::{
                DATADIR_GET, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
            },
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
    get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    get_data_here:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    query_get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC) -> HRESULT,
    get_canonical_format_etc:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut FORMATETC) -> HRESULT,
    set_data:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *const STGMEDIUM, i32) -> HRESULT,
    enum_format_etc: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT,
    d_advise: unsafe extern "system" fn(
        *mut c_void,
        *const FORMATETC,
        u32,
        *mut c_void,
        *mut u32,
    ) -> HRESULT,
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
        // SAFETY: OleInitialize is called once per drag on the thread that will
        // call DoDragDrop. S_FALSE means OLE was already initialized on this
        // thread and is safe to use. RPC_E_CHANGED_MODE means another COM
        // apartment model is active; we abort rather than risk mismatching
        // apartment states.
        let init_hr = OleInitialize(null());
        let initialized = init_hr == S_OK || init_hr == S_FALSE;
        if !initialized && init_hr != RPC_E_CHANGED_MODE {
            return Err(format!("OleInitialize failed: 0x{:08X}", init_hr as u32));
        }
        if init_hr == RPC_E_CHANGED_MODE {
            return Err("OLE drag-and-drop unavailable on this host UI thread".to_string());
        }

        // SAFETY: The COM objects are stack-allocated and live for the entire
        // DoDragDrop call. Their vtables are static and correctly implement
        // IUnknown/IDataObject and IUnknown/IDropSource. The cast to *mut c_void
        // is the expected interface pointer type for DoDragDrop.
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

        // SAFETY: Matches the successful OleInitialize/OleInitialize call above.
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

    // SAFETY: `this` is a valid IDataObject pointer passed by OLE. The caller
    // provides a non-null `iid` and a writable `interface` pointer. We only
    // expose IUnknown and IDataObject.
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
    // SAFETY: `this` points to a MidiFileDataObject whose first field is the
    // IDataObject vtable pointer. This function is only called through that
    // vtable by OLE.
    let object = this as *mut MidiFileDataObject;
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn data_release(this: *mut c_void) -> u32 {
    // SAFETY: Same invariant as data_add_ref. The object is stack-allocated in
    // start_midi_file_drag, so the count never reaches zero in practice; this
    // implementation is correct for a heap-allocated future version.
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

    // SAFETY: `this` is a valid IDataObject pointer; we convert it to the
    // underlying MidiFileDataObject to read the path. build_hdrop_medium copies
    // the path into a newly allocated HGLOBAL, so the borrow ends before the
    // function returns.
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

    // SAFETY: Same invariant as data_query_interface; `this` is a valid
    // IDropSource pointer. We expose IUnknown and IDropSource.
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
    // SAFETY: `this` points to a MidiFileDropSource whose first field is the
    // IDropSource vtable pointer. This function is only called through that
    // vtable by OLE.
    let object = this as *mut MidiFileDropSource;
    (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn source_release(this: *mut c_void) -> u32 {
    // SAFETY: Same invariant as source_add_ref. The object is stack-allocated
    // in start_midi_file_drag.
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
    // SAFETY: We allocate a single HGLOBAL block large enough for the
    // DROPFILES header plus the UTF-16 path including a terminating zero. The
    // caller owns the returned STGMEDIUM and is responsible for freeing the
    // HGLOBAL through the standard OLE medium-release rules.
    let path_bytes = (path_wide.len() + 1) * size_of::<u16>();
    let dropfiles_size = size_of::<DROPFILES>();
    let total_size = dropfiles_size + path_bytes;

    let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size);
    if hglobal.is_null() {
        return Err(DV_E_FORMATETC);
    }

    // SAFETY: GlobalLock returns a writable pointer to the committed block.
    // We checked for null and the allocation size matches the accesses below.
    let memory = GlobalLock(hglobal);
    if memory.is_null() {
        GlobalFree(hglobal);
        return Err(DV_E_FORMATETC);
    }

    let dropfiles = memory as *mut DROPFILES;
    (*dropfiles).pFiles = dropfiles_size as u32;
    (*dropfiles).fWide = 1;

    // SAFETY: `file_list` points past the header into the remaining allocated
    // bytes (path_bytes). copy_nonoverlapping copies `path_wide.len()` elements,
    // and we write the terminator at index `path_wide.len()`, which is within
    // the allocated `path_wide.len() + 1` elements.
    let file_list = (memory as *mut u8).add(dropfiles_size) as *mut u16;
    copy_nonoverlapping(path_wide.as_ptr(), file_list, path_wide.len());
    *file_list.add(path_wide.len()) = 0;

    // SAFETY: GlobalUnlock decrements the lock count; this is the matching
    // unlock for the GlobalLock above.
    GlobalUnlock(hglobal);

    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL as u32,
        u: STGMEDIUM_0 { hGlobal: hglobal },
        pUnkForRelease: null_mut(),
    })
}

unsafe fn guid_eq(iid: *const GUID, expected: &GUID) -> bool {
    // SAFETY: The caller (OLE COM machinery) always provides a valid non-null
    // GUID pointer for QueryInterface.
    if iid.is_null() {
        return false;
    }
    let iid = &*iid;
    iid.data1 == expected.data1
        && iid.data2 == expected.data2
        && iid.data3 == expected.data3
        && iid.data4 == expected.data4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hdrop_medium_produces_valid_global_medium() {
        // Path encoded as a null-terminated wide-char sequence, matching what
        // start_midi_file_drag builds from an OsStr.
        let mut path_wide: Vec<u16> = std::path::Path::new(r"C:\\temp\\drag.mid")
            .as_os_str()
            .encode_wide()
            .collect();
        path_wide.push(0);

        let medium = unsafe { build_hdrop_medium(&path_wide) };
        assert!(
            medium.is_ok(),
            "build_hdrop_medium should succeed for a valid path"
        );

        let medium = medium.unwrap();
        assert_eq!(medium.tymed, TYMED_HGLOBAL as u32);

        // SAFETY: `STGMEDIUM.u` is a C union; we only read the `hGlobal` member
        // because we set `tymed` to `TYMED_HGLOBAL` above.
        let hglobal = unsafe { medium.u.hGlobal };
        assert!(!hglobal.is_null(), "HGLOBAL should be allocated");

        // SAFETY: We own the HGLOBAL returned by build_hdrop_medium. Verify it
        // contains a DROPFILES structure with the expected header and the path.
        // DROPFILES is a packed struct, so fields are read with read_unaligned.
        unsafe {
            let memory = GlobalLock(hglobal);
            assert!(!memory.is_null(), "HGLOBAL should be lockable");

            let dropfiles = memory as *const DROPFILES;
            let p_files = std::ptr::addr_of!((*dropfiles).pFiles).read_unaligned();
            let f_wide = std::ptr::addr_of!((*dropfiles).fWide).read_unaligned();
            assert_eq!(p_files, size_of::<DROPFILES>() as u32);
            assert_eq!(f_wide, 1);

            let file_list = (memory as *const u8).add(size_of::<DROPFILES>()) as *const u16;
            let mut len = 0usize;
            while *file_list.add(len) != 0 {
                len += 1;
            }
            let stored: Vec<u16> = std::slice::from_raw_parts(file_list, len).to_vec();
            assert_eq!(stored, path_wide[..path_wide.len() - 1]);

            GlobalUnlock(hglobal);
            GlobalFree(hglobal);
        }
    }
}
