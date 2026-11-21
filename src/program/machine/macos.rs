//! The listeners of the person's processes on macOS: libproc lists the processes, their
//! descriptors and, for a socket, what it is. The layout of `struct socket_fdinfo` is read from the
//! system header (as bateri does, whose `ports.rs` this is made from).

use std::ffi::{CStr, c_int, c_void};

use super::{Listener, display_name};

/// `struct socket_fdinfo` (`<sys/proc_info.h>`) is not in `libc`; the offsets below were read from
/// the header with `offsetof` (arm64 and x86_64 share the layout: fixed-size integers, no
/// pointers). A reply whose size is not the struct's is refused, so a changed layout reads as "no
/// socket", never as a wrong port.
const PROC_PIDFDSOCKETINFO: c_int = 3;
const SOCKET_FDINFO_SIZE: usize = 792;
const SOI_KIND: usize = 256;
const SOCKINFO_TCP: i32 = 2;
const INSI_LPORT: usize = 268;
const TCPSI_STATE: usize = 344;
const TSI_S_LISTEN: i32 = 1;

/// `PROC_ALL_PIDS` (`<sys/proc_info.h>`, not in `libc`).
const PROC_ALL_PIDS: u32 = 1;
/// `KERN_PROCARGS2` (`<sys/sysctl.h>`).
const KERN_PROCARGS2: c_int = 49;

pub fn listeners() -> Vec<Listener> {
    // SAFETY: `geteuid` takes nothing and cannot fail.
    let me = unsafe { libc::geteuid() };
    let mut found = Vec::new();
    for pid in pids() {
        if owner(pid) != Some(me) {
            continue;
        }
        let ports = listening(pid);
        if ports.is_empty() {
            continue;
        }
        let name = name_of(pid).unwrap_or_else(|| pid.to_string());
        for port in ports {
            found.push(Listener {
                port,
                name: name.clone(),
            });
        }
    }
    found
}

/// Every process there is.
fn pids() -> Vec<c_int> {
    // SAFETY: a null buffer of size zero: the call returns the bytes it needs.
    let needed = unsafe { libc::proc_listpids(PROC_ALL_PIDS, 0, std::ptr::null_mut(), 0) };
    let Ok(needed) = usize::try_from(needed) else {
        return Vec::new();
    };
    // Room for processes born between the two calls.
    let capacity = needed / size_of::<c_int>() + 32;
    let mut pids: Vec<c_int> = vec![0; capacity];
    let Ok(bytes) = c_int::try_from(capacity * size_of::<c_int>()) else {
        return Vec::new();
    };
    // SAFETY: the buffer is `capacity` pids long and belongs to this frame; the size passed is
    // exactly its byte length.
    let filled = unsafe { libc::proc_listpids(PROC_ALL_PIDS, 0, pids.as_mut_ptr().cast(), bytes) };
    let count = usize::try_from(filled).unwrap_or(0) / size_of::<c_int>();
    pids.truncate(count.min(capacity));
    pids.retain(|pid| *pid > 0);
    pids
}

/// The user a process belongs to.
fn owner(pid: c_int) -> Option<u32> {
    // SAFETY: `proc_bsdinfo` is a plain C struct of integers and `c_char` arrays; all zero bytes
    // is a valid value of it.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = c_int::try_from(size_of::<libc::proc_bsdinfo>()).ok()?;
    // SAFETY: the buffer is one `proc_bsdinfo` and the size passed is its own.
    let got = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            std::ptr::addr_of_mut!(info).cast::<c_void>(),
            size,
        )
    };
    (got == size).then_some(info.pbi_uid)
}

/// The TCP ports `pid` listens on; empty for a process we cannot read or that is gone.
fn listening(pid: c_int) -> Vec<u16> {
    // SAFETY: a null buffer of size zero: the call returns the bytes it needs.
    let needed =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    let Ok(needed) = usize::try_from(needed) else {
        return Vec::new();
    };
    if needed == 0 {
        return Vec::new();
    }
    // Room for descriptors opened between the two calls.
    let capacity = needed / size_of::<libc::proc_fdinfo>() + 16;
    let mut fds: Vec<libc::proc_fdinfo> = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        capacity
    ];
    let Ok(bytes) = c_int::try_from(capacity * size_of::<libc::proc_fdinfo>()) else {
        return Vec::new();
    };
    // SAFETY: the buffer is `capacity` records long and belongs to this frame; the size passed is
    // exactly its byte length.
    let filled = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast(),
            bytes,
        )
    };
    let count = usize::try_from(filled).unwrap_or(0) / size_of::<libc::proc_fdinfo>();
    fds.truncate(count.min(capacity));

    let mut found = Vec::new();
    let mut info = [0u8; SOCKET_FDINFO_SIZE];
    for fd in fds {
        if fd.proc_fdtype != libc::PROX_FDTYPE_SOCKET as u32 {
            continue;
        }
        // SAFETY: the buffer belongs to this frame and its size goes to the call as is; the kernel
        // writes at most that many bytes.
        let written = unsafe {
            libc::proc_pidfdinfo(
                pid,
                fd.proc_fd,
                PROC_PIDFDSOCKETINFO,
                info.as_mut_ptr().cast(),
                SOCKET_FDINFO_SIZE as c_int,
            )
        };
        if usize::try_from(written).ok() != Some(SOCKET_FDINFO_SIZE) {
            continue;
        }
        let int =
            |at: usize| i32::from_ne_bytes([info[at], info[at + 1], info[at + 2], info[at + 3]]);
        if int(SOI_KIND) != SOCKINFO_TCP || int(TCPSI_STATE) != TSI_S_LISTEN {
            continue;
        }
        // `insi_lport` holds the port in network order in its first two bytes (lsof's
        // `ntohs((u_short)insi_lport)`).
        let port = u16::from_be_bytes([info[INSI_LPORT], info[INSI_LPORT + 1]]);
        if port != 0 {
            found.push(port);
        }
    }
    found
}

/// The process as `ps` shows it: from its arguments (`sysctl kern.procargs2`), else the kernel's
/// name for it.
fn name_of(pid: c_int) -> Option<String> {
    let args = arguments(pid).unwrap_or_default();
    let mut kernel = [0u8; 64];
    // SAFETY: the buffer belongs to this frame and its size goes to the call as is.
    let got = unsafe { libc::proc_name(pid, kernel.as_mut_ptr().cast(), kernel.len() as u32) };
    let kernel = (got > 0)
        .then(|| CStr::from_bytes_until_nul(&kernel).ok())
        .flatten()
        .map(|name| name.to_string_lossy().into_owned());
    display_name(&args, kernel.as_deref())
}

/// The arguments of a process: `KERN_PROCARGS2` is the argument count (a C int), the path of the
/// executable, the nul bytes that pad it, then the arguments, each ended by a nul.
fn arguments(pid: c_int) -> Option<Vec<String>> {
    let mut mib = [libc::CTL_KERN, KERN_PROCARGS2, pid];
    let mut size = 0_usize;
    // SAFETY: asks only for the size the arguments need; the name has three words, as passed.
    let asked = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &raw mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if asked != 0 || size < 8 {
        return None;
    }
    let mut buffer = vec![0u8; size];
    // SAFETY: the buffer is `size` bytes, which is what is passed.
    let read = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            buffer.as_mut_ptr().cast(),
            &raw mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 {
        return None;
    }
    buffer.truncate(size);
    let count = usize::try_from(i32::from_ne_bytes(buffer.get(..4)?.try_into().ok()?)).ok()?;
    let rest = buffer.get(4..)?;
    // The executable's path, then the padding.
    let after_path = rest.iter().position(|byte| *byte == 0)?;
    let rest = &rest[after_path..];
    let start = rest.iter().position(|byte| *byte != 0)?;
    let args = rest[start..]
        .split(|byte| *byte == 0)
        .take(count)
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect();
    Some(args)
}
