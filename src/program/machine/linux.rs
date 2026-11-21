//! The listeners of the person's processes on Linux: the `LISTEN` rows of `/proc/net/tcp` and
//! `tcp6`, matched by inode to the sockets in each of the person's processes' `/proc/<pid>/fd`.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::MetadataExt as _;

use super::{Listener, display_name};

pub fn listeners() -> Vec<Listener> {
    let table = listen_table();
    if table.is_empty() {
        return Vec::new();
    }
    // SAFETY: `geteuid` takes nothing and cannot fail.
    let me = unsafe { libc::geteuid() };
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in dir.filter_map(Result::ok) {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if entry.metadata().map(|meta| meta.uid()).ok() != Some(me) {
            continue;
        }
        let ports = ports_of(pid, &table);
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

/// The ports that listen, by the inode of their socket.
fn listen_table() -> BTreeMap<u64, u16> {
    let mut table = BTreeMap::new();
    for file in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for row in text.lines().skip(1).filter_map(parse_row) {
            table.insert(row.inode, row.port);
        }
    }
    table
}

/// The ports of the sockets `pid` holds that listen.
fn ports_of(pid: u32, table: &BTreeMap<u64, u16>) -> BTreeSet<u16> {
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        return BTreeSet::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_link(entry.path()).ok())
        .filter_map(|target| socket_inode(&target.to_string_lossy()))
        .filter_map(|inode| table.get(&inode).copied())
        .collect()
}

fn name_of(pid: u32) -> Option<String> {
    let args: Vec<String> = std::fs::read(format!("/proc/{pid}/cmdline"))
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|arg| !arg.is_empty())
                .map(|arg| String::from_utf8_lossy(arg).into_owned())
                .collect()
        })
        .unwrap_or_default();
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok();
    display_name(&args, comm.as_deref().map(str::trim))
}

/// `socket:[12345]` gives `12345`.
fn socket_inode(link: &str) -> Option<u64> {
    link.strip_prefix("socket:[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

#[derive(Debug, PartialEq, Eq)]
struct Row {
    port: u16,
    inode: u64,
}

/// A `LISTEN` row of `/proc/net/tcp{,6}`: `  0: 0100007F:0BB8 00000000:0000 0A … 12345 …`, the
/// local port (field 1, after the colon, hex), the state (`0A`) and the inode (field 9).
fn parse_row(line: &str) -> Option<Row> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if *fields.get(3)? != "0A" {
        return None;
    }
    let port = u16::from_str_radix(fields.get(1)?.split_once(':')?.1, 16).ok()?;
    let inode = fields.get(9)?.parse().ok()?;
    (port != 0 && inode != 0).then_some(Row { port, inode })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listening_row_gives_its_port_and_inode_and_another_state_gives_none() {
        let row = "   0: 0100007F:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 12345 1 0000000000000000 100 0 0 10 0";
        assert_eq!(
            parse_row(row),
            Some(Row {
                port: 3000,
                inode: 12345
            })
        );
        let established = row.replace(" 0A ", " 01 ");
        assert_eq!(parse_row(&established), None);
        assert_eq!(parse_row("garbage"), None);
    }

    #[test]
    fn a_socket_link_is_its_inode() {
        assert_eq!(socket_inode("socket:[12345]"), Some(12345));
        assert_eq!(socket_inode("/dev/null"), None);
    }
}
