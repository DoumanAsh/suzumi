use std::net::IpAddr;
use std::{io, time};

pub struct Pinger {
    addr: IpAddr,
}

impl Pinger {
    ///Creates new instance
    ///
    ///`addr` - Destination address for the ICMP packet.
    pub const fn new(addr: IpAddr) -> Self {
        Self {
            addr,
        }
    }

    #[inline]
    pub fn ping(&self) -> io::Result<time::Duration> {
        self.ping_timeout(time::Duration::from_millis(250))
    }

    pub fn ping_timeout(&self, timeout: time::Duration) -> io::Result<time::Duration> {
        let addr = std::net::SocketAddr::new(self.addr, 53);

        let before = time::Instant::now();
        match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => Ok(time::Instant::now().duration_since(before)),
            Err(error) => {
                rogu::debug!("Ping fail: {}", error);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pinger;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn pinger_should_ping_google_dns() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let pinger = Pinger::new(ip);

        let duration = pinger.ping().expect("To ping");
        assert_ne!(duration.as_millis(), 0);
    }

    #[test]
    fn pinger_should_ping_non_existing() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 9));
        let pinger = Pinger::new(ip);

        pinger.ping().expect_err("Fail to ping");
    }
}
