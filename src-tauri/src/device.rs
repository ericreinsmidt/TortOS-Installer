// A card opened for writing and reading back, as each system allows it.
//
// macOS: the card's volumes are unmounted, and its raw disk (/dev/rdiskN) is
// opened through authopen, Apple's tool for exactly this: it asks for the
// password and hands back the open disk, so the installer never runs as root.
//
// Windows: the installer runs as administrator (its manifest), and every
// volume on the card is locked or dismounted first, found by disk rather
// than by drive letter: a volume without a letter is still mounted, and
// Windows refuses writes into a mounted volume's sectors.
//
// Linux: the window never runs as root. The installer runs itself again
// through pkexec in a helper mode with no window (helper.rs), and that opens
// the card here, directly.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

use crate::writer::Card;

pub struct DeviceCard {
    file: File,
    /// The card's volumes, locked until the card is closed. After `file`,
    /// so the disk closes before the volumes are let go.
    #[cfg(windows)]
    _locks: windows_volumes::Locked,
}

impl Read for DeviceCard {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for DeviceCard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for DeviceCard {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Card for DeviceCard {
    fn sync(&mut self) -> io::Result<()> {
        // A Mac's raw disk refuses fsync; it was opened O_SYNC, so each write
        // was on the card when it returned
        #[cfg(target_os = "macos")]
        {
            let _ = self.file.sync_all();
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        self.file.sync_all()
    }

    /// Linux keeps a block device's pages in memory; without this the read-back
    /// would read back the cache. A Mac's raw disk and a Windows physical
    /// drive have none.
    #[cfg(target_os = "linux")]
    fn drop_cache(&mut self) -> io::Result<()> {
        use std::os::unix::io::AsRawFd;
        const BLKFLSBUF: libc::c_ulong = 0x1261;
        if unsafe { libc::ioctl(self.file.as_raw_fd(), BLKFLSBUF as _, 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::os::unix::io::{FromRawFd, RawFd};
    use std::os::unix::net::UnixStream;
    use std::process::{Command, Stdio};

    /// Takes the card's volumes off the desktop, so nothing else writes to
    /// them while the disk is written.
    pub fn unmount(disk: &str) -> Result<(), String> {
        let ok = |force: bool| {
            let mut c = Command::new("diskutil");
            c.arg("unmountDisk");
            if force {
                c.arg("force");
            }
            c.arg(disk).output().is_ok_and(|o| o.status.success())
        };
        if ok(false) || ok(true) {
            Ok(())
        } else {
            Err(format!("Couldn't unmount {disk}. Close anything that has the card open, and try again."))
        }
    }

    /// The file descriptor authopen sends over the socket, with SCM_RIGHTS.
    fn receive_fd(sock: &UnixStream) -> io::Result<RawFd> {
        use std::os::unix::io::AsRawFd;
        let mut byte = [0u8; 1];
        let mut iov = libc::iovec { iov_base: byte.as_mut_ptr().cast(), iov_len: 1 };
        // u64s, so the control header lands aligned
        let mut control = [0u64; 8];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = std::mem::size_of_val(&control) as _;
        let n = unsafe { libc::recvmsg(sock.as_raw_fd(), &mut msg, 0) };
        if n <= 0 {
            return Err(io::Error::other("authopen sent nothing"));
        }
        let mut c = unsafe { libc::CMSG_FIRSTHDR(&msg) };
        while !c.is_null() {
            let h = unsafe { &*c };
            if h.cmsg_level == libc::SOL_SOCKET && h.cmsg_type == libc::SCM_RIGHTS {
                let data = unsafe { libc::CMSG_DATA(c) } as *const RawFd;
                return Ok(unsafe { std::ptr::read_unaligned(data) });
            }
            c = unsafe { libc::CMSG_NXTHDR(&msg, c) };
        }
        Err(io::Error::other("authopen sent no disk"))
    }

    pub fn open(disk: &str) -> Result<DeviceCard, String> {
        unmount(disk)?;
        let raw = format!("/dev/r{disk}");
        let (ours, theirs) = UnixStream::pair().map_err(|e| format!("Opening the card: {e}"))?;
        let theirs_fd = std::os::unix::io::IntoRawFd::into_raw_fd(theirs);
        let mut child = Command::new("/usr/libexec/authopen")
            .args(["-stdoutpipe", "-o", &(libc::O_RDWR | libc::O_SYNC).to_string(), &raw])
            .stdout(unsafe { Stdio::from_raw_fd(theirs_fd) })
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Couldn't run authopen: {e}"))?;
        let fd = receive_fd(&ours);
        let status = child.wait().map_err(|e| format!("authopen: {e}"))?;
        match fd {
            Ok(fd) if status.success() => Ok(DeviceCard { file: unsafe { File::from_raw_fd(fd) } }),
            _ => Err("The card wasn't opened: the password was cancelled or not accepted.".into()),
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::os::unix::fs::OpenOptionsExt;
    use std::process::Command;

    /// Every mounted partition of the card unmounted (as root, in the helper).
    pub fn unmount(dev: &str) -> Result<(), String> {
        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        for line in mounts.lines() {
            let mut f = line.split_whitespace();
            let (Some(src), Some(dir)) = (f.next(), f.next()) else { continue };
            if is_on(src, dev) {
                let ok = Command::new("umount").arg(src).status().is_ok_and(|s| s.success());
                if !ok {
                    return Err(format!("Couldn't unmount {src} from {dir}. Close anything that has it open."));
                }
            }
        }
        Ok(())
    }

    /// The disk itself or one of its partitions: /dev/sdb1, and /dev/mmcblk0p1
    /// where the disk's name ends in a digit. Never /dev/loop10 for /dev/loop1.
    pub(super) fn is_on(src: &str, dev: &str) -> bool {
        let Some(rest) = src.strip_prefix(dev) else { return false };
        let number = if dev.ends_with(|c: char| c.is_ascii_digit()) { rest.strip_prefix('p') } else { Some(rest) };
        rest.is_empty() || number.is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    }

    pub fn open(dev: &str) -> Result<DeviceCard, String> {
        unmount(dev)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(dev)
            .map_err(|e| format!("Opening {dev}: {e}"))?;
        Ok(DeviceCard { file })
    }
}

#[cfg(windows)]
mod windows_volumes {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, FILE_FLAGS_AND_ATTRIBUTES,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME};
    use windows::Win32::System::IO::DeviceIoControl;

    /// CTL_CODE(IOCTL_VOLUME_BASE 0x56, 0, METHOD_BUFFERED, FILE_ANY_ACCESS):
    /// winioctl.h's value, which this version of the windows crate leaves out
    const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x0056_0000;

    pub struct Locked(Vec<HANDLE>);

    impl Drop for Locked {
        fn drop(&mut self) {
            for &h in &self.0 {
                let mut n = 0u32;
                unsafe {
                    let _ = DeviceIoControl(h, FSCTL_UNLOCK_VOLUME, None, 0, None, 0, Some(&mut n), None);
                    let _ = CloseHandle(h);
                }
            }
        }
    }

    fn open(path: &[u16], access: u32) -> Option<HANDLE> {
        unsafe {
            CreateFileW(PCWSTR(path.as_ptr()), access, FILE_SHARE_READ | FILE_SHARE_WRITE, None,
                        OPEN_EXISTING, FILE_FLAGS_AND_ATTRIBUTES(0), None)
        }
        .ok()
    }

    /// Whether any of a volume's extents lies on the disk.
    fn on_disk(h: HANDLE, disk: u32) -> bool {
        // VOLUME_DISK_EXTENTS: a count, then 24-byte extents from byte 8, each
        // starting with its disk number; u64s for the alignment
        let mut buf = [0u64; 64];
        let mut n = 0u32;
        let ok = unsafe {
            DeviceIoControl(h, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, None, 0, Some(buf.as_mut_ptr().cast()),
                            std::mem::size_of_val(&buf) as u32, Some(&mut n), None)
        }
        .is_ok();
        if !ok {
            return false;
        }
        let count = (buf[0] & 0xffff_ffff) as usize;
        (0..count.min(21)).any(|i| (buf[1 + 3 * i] & 0xffff_ffff) as u32 == disk)
    }

    /// Every volume on the disk locked or, failing that, dismounted.
    pub fn lock(disk: u32) -> Result<Locked, String> {
        let mut locked = Locked(Vec::new());
        let mut name = [0u16; 260];
        let find = unsafe { FindFirstVolumeW(&mut name) }.map_err(|e| format!("Listing volumes: {e}"))?;
        let result = loop {
            let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            // \\?\Volume{...}\ opens as a device without its last backslash
            let mut path: Vec<u16> = name[..len].to_vec();
            if path.last() == Some(&u16::from(b'\\')) {
                path.pop();
            }
            path.push(0);
            // Asked with no access at all, so other disks' volumes are only
            // looked at
            let ours = open(&path, 0).is_some_and(|h| {
                let ours = on_disk(h, disk);
                unsafe { let _ = CloseHandle(h); }
                ours
            });
            if ours {
                let Some(h) = open(&path, FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0) else {
                    break Err("Couldn't open one of the card's volumes".to_string());
                };
                let mut n = 0u32;
                let lock = unsafe { DeviceIoControl(h, FSCTL_LOCK_VOLUME, None, 0, None, 0, Some(&mut n), None) }.is_ok();
                let dismount = unsafe { DeviceIoControl(h, FSCTL_DISMOUNT_VOLUME, None, 0, None, 0, Some(&mut n), None) }.is_ok();
                locked.0.push(h);
                // Either lets the writes in; with neither, Windows would refuse
                // them partway through
                if !lock && !dismount {
                    break Err("Couldn't lock the card. Close anything that has it open, and try again.".into());
                }
            }
            if unsafe { FindNextVolumeW(find, &mut name) }.is_err() {
                break Ok(());
            }
        };
        unsafe { let _ = FindVolumeClose(find); }
        result.map(|_| locked)
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::os::windows::io::FromRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_WRITE_THROUGH, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_MODE, OPEN_EXISTING,
    };

    /// `id` is \\.\PhysicalDriveN.
    pub fn open(id: &str) -> Result<DeviceCard, String> {
        let disk: u32 = id
            .strip_prefix("\\\\.\\PhysicalDrive")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| format!("Not a disk: {id}"))?;
        let locks = windows_volumes::lock(disk)?;
        let path: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        let h = unsafe {
            CreateFileW(PCWSTR(path.as_ptr()), FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0, FILE_SHARE_MODE(0),
                        None, OPEN_EXISTING, FILE_FLAG_WRITE_THROUGH, None)
        }
        .map_err(|e| format!("Opening the card: {e}"))?;
        let file = unsafe { File::from_raw_handle(h.0) };
        Ok(DeviceCard { file, _locks: locks })
    }
}

/// The card for writing. On Linux only the helper, running as root, calls it.
pub fn open(id: &str) -> Result<DeviceCard, String> {
    #[cfg(target_os = "macos")]
    return mac::open(id);
    #[cfg(target_os = "linux")]
    return linux::open(id);
    #[cfg(windows)]
    return win::open(id);
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    #[test]
    fn unmounts_only_the_cards_own_partitions() {
        let on = super::linux::is_on;
        assert!(on("/dev/sdb", "/dev/sdb") && on("/dev/sdb1", "/dev/sdb") && on("/dev/sdb12", "/dev/sdb"));
        assert!(on("/dev/mmcblk0p1", "/dev/mmcblk0") && on("/dev/loop1p2", "/dev/loop1"));
        assert!(!on("/dev/sdba", "/dev/sdb") && !on("/dev/loop10", "/dev/loop1") && !on("/dev/mmcblk01", "/dev/mmcblk0"));
        assert!(!on("/dev/nvme0n10p1", "/dev/nvme0n1") && !on("/dev/loop1p", "/dev/loop1"));
    }
}
