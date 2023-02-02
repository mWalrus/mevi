use anyhow::Result;
use libc::SHM_RDONLY;
use std::{mem, ptr};
use x11rb::{
    connection::Connection,
    image::Image,
    protocol::{
        self,
        shm::{self, ConnectionExt as _},
        xproto::{ConnectionExt, CreateGCAux, Screen, Window},
    },
};

// TODO: impl Drop
pub struct SHMInfo {
    pub seg: u32,
    pub id: i32,
    pub addr: *const u8,
}

struct Bgr {
    _b: u8,
    _g: u8,
    _r: u8,
    _padding: u8,
}

pub fn attach_image<'a, C: Connection>(
    conn: &'a C,
    img: &'a Image,
    s: &Screen,
    win: Window,
) -> Result<SHMInfo> {
    let (w, h) = (img.width(), img.height());
    let size = w as usize * h as usize * mem::size_of::<Bgr>();
    let shminfo = unsafe {
        let id = libc::shmget(libc::IPC_PRIVATE, size, libc::IPC_CREAT | 0o777);
        if id < 0 {
            mevi_err!("shmget failed");
            std::process::exit(1);
        }

        let addr = libc::shmat(id, ptr::null(), SHM_RDONLY);
        let seg = conn.generate_id()?;

        conn.shm_attach(seg, id as u32, false)?;
        SHMInfo {
            seg,
            id,
            addr: addr as *const u8,
        }
    };

    conn.shm_attach(shminfo.seg, shminfo.id as u32, false)?;

    let pm = conn.generate_id()?;
    let gc = conn.generate_id()?;

    conn.create_gc(gc, s.root, &CreateGCAux::default().graphics_exposures(0))?;
    mevi_info!("created graphics context: {gc}");

    shm::create_pixmap(conn, pm, win, 0, 0, s.root_depth, shminfo.seg, 0)?;
    mevi_info!("created shared pixmap: {pm}");

    shm::put_image(
        conn,
        pm,
        gc,
        w,
        h,
        0,
        0,
        w,
        h,
        0,
        0,
        s.root_depth,
        2,
        false,
        shminfo.seg,
        0,
    )?;
    Ok(shminfo)
}

pub fn check_shm_version<'a, C: Connection>(conn: &C) -> Result<Option<(u16, u16)>> {
    if conn
        .extension_information(protocol::shm::X11_EXTENSION_NAME)?
        .is_none()
    {
        return Ok(None);
    }
    let v = conn.shm_query_version()?.reply()?;

    if v.shared_pixmaps {
        return Ok(None);
    }

    Ok(Some((v.major_version, v.minor_version)))
}
