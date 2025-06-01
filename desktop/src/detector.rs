use ipnetwork::IpNetwork;
use smol::{lock::Semaphore, net::UdpSocket};
use smol_timeout::TimeoutExt;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use crate::{Error, Result, config, device::Device};

pub struct DeviceDetector {
    subnet: IpNetwork,
    port: u16,
    recv_timeout: Duration,
    send_buf: [u8; 2],
}

impl DeviceDetector {
    #[allow(unused)]
    pub fn new(subnet: IpNetwork, port: u16, recv_timeout: Duration) -> Self {
        Self {
            subnet,
            port,
            recv_timeout,
            send_buf: [0x01, 0x01],
        }
    }

    pub fn with_subnet(subnet: IpNetwork) -> Self {
        Self {
            subnet,
            ..Default::default()
        }
    }

    pub async fn run_with_timeout(self, timeout: Duration) -> Result<Vec<Device>> {
        self.run()
            .timeout(timeout)
            .await
            .ok_or(Error::FutureTimeout)?
    }

    pub async fn run(self) -> Result<Vec<Device>> {
        let hosts: Vec<IpAddr> = self.subnet.iter().collect();
        let sem = Arc::new(Semaphore::new(256));

        let tasks: Vec<_> = hosts
            .into_iter()
            .map(|ip| self.detector_worker(Arc::clone(&sem), ip))
            .collect();

        let results = futures::future::join_all(tasks).await;

        let open_devices: Vec<_> = results
            .into_iter()
            .filter(|(_, open)| *open)
            .map(|(ip, _)| Device::new(ip, self.port))
            .collect();

        Ok(open_devices)
    }

    async fn detector_worker(&self, sem: Arc<Semaphore>, ip: IpAddr) -> (IpAddr, bool) {
        let _permit = sem.acquire().await;
        let addr = SocketAddr::new(ip, self.port);
        let open = Self::detect_device(&self.send_buf, addr, self.recv_timeout)
            .await
            .unwrap_or(false);
        (ip, open)
    }

    async fn detect_device(send_buf: &[u8], addr: SocketAddr, timeout: Duration) -> Result<bool> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(Error::UdpConnect)?;
        socket.connect(addr).await.map_err(Error::UdpConnect)?;
        let mut recv_buf = [0u8; 8];
        socket.send(send_buf).await.map_err(Error::UdpSend)?;
        let size = socket
            .recv(&mut recv_buf)
            .timeout(timeout)
            .await
            .ok_or(Error::FutureTimeout)?
            .map_err(Error::UdpRecv)?;
        let requirements = [size >= 2, recv_buf[0] == 0x01, recv_buf[1] == 0x01];
        if requirements.iter().all(|p| *p) {
            return Ok(true);
        }
        Ok(false)
    }
}

impl Default for DeviceDetector {
    fn default() -> Self {
        Self {
            subnet: "192.168.1.0/24".parse().unwrap(),
            port: config::DEVICE_PORT,
            recv_timeout: Duration::from_millis(500),
            send_buf: [0x01, 0x01],
        }
    }
}
