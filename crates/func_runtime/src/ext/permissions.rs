use ::deno_fetch::FetchPermissions;
use ::deno_net::NetPermissions;
use ::deno_web::TimersPermission;
use deno_core::url::{Host, Url};
use deno_permissions::{CheckedPath, OpenAccessKind, PermissionCheckError, PermissionDeniedError};
use deno_websocket::WebSocketPermissions;
use std::borrow::Cow;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::Path;

macro_rules! deny {
    ($access:expr, $name:expr) => {
        Err(PermissionCheckError::PermissionDenied(
            PermissionDeniedError {
                access: $access.to_string(),
                name: $name,
                custom_message: None,
            },
        ))
    };
}

pub struct Permissions {}

impl WebSocketPermissions for Permissions {
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }
}

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
}

impl FetchPermissions for Permissions {
    fn check_net(
        &mut self,
        _host: &str,
        _port: u16,
        _api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_net_url(&mut self, url: &Url, api_name: &str) -> Result<(), PermissionCheckError> {
        match url.host() {
            Some(host) => {
                if host.to_string().is_empty() {
                    return deny!(api_name, "fetch_net_url");
                }

                tracing::debug!("checking net for {:?} (api: {})", url, api_name);

                match host {
                    Host::Ipv4(addr) => return check_addr(addr.into(), api_name),
                    Host::Ipv6(addr) => return check_addr(addr.into(), api_name),
                    // TODO: this resolution will block the thread, we may want to remove.
                    // Alternatively we can intercept at the DNS resolver for fetch.
                    Host::Domain(domain) => match (domain, 80).to_socket_addrs() {
                        Ok(addrs) => {
                            for addr in addrs {
                                tracing::info!("resolved {} to {}", domain, addr);
                                check_addr(addr.ip(), api_name)?;
                            }
                        }
                        Err(err) => {
                            tracing::error!("Failed to resolve domain {}: {}", domain, err);
                            return deny!(api_name, "fetch_net_url");
                        }
                    },
                }

                Ok(())
            }
            None => deny!(api_name, "fetch_net_url"),
        }
    }

    fn check_open<'a>(
        &mut self,
        _path: Cow<'a, Path>,
        _open_access: OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        deny!(api_name, "fetch_open")
    }

    fn check_net_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        deny!(api_name, "fetch_net_vsock")
    }
}

impl NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        // TODO: need to implement
        deny!(api_name, "net")
    }

    fn check_open<'a>(
        &mut self,
        _path: Cow<'a, Path>,
        _open_access: OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        deny!(api_name, "net_open")
    }

    fn check_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        deny!(api_name, "net_vsock")
    }
}

/// Check if the given address is allowed. This is a defense in depth measure to prevent
/// unauthorized access to network resources. There should be additional measures in place
/// to protect network resources.
fn check_addr(addr: IpAddr, api_name: &str) -> Result<(), PermissionCheckError> {
    match addr {
        IpAddr::V4(addr) => {
            if addr.is_unspecified()
                || addr.is_loopback()
                || addr.is_private()
                || addr.is_link_local()
                || addr.is_broadcast()
            {
                return deny!(api_name, "net_addr");
            }
        }
        IpAddr::V6(addr) => {
            if addr.is_unspecified()
                || addr.is_loopback()
                || addr.is_unique_local()
                || addr.is_unicast_link_local()
            {
                return deny!(api_name, "net_addr");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    const TEST_API_NAME: &str = "test()";

    #[test]
    fn test_timers_permission_refute_hrtime() {
        let mut permissions = Permissions {};
        assert!(!permissions.allow_hrtime());
    }

    #[test]
    fn test_fetch_permissions_check_net_url_no_host() {
        let mut permissions = Permissions {};
        let url = Url::parse("file:///test").unwrap();
        let result = permissions.check_net_url(&url, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "fetch_net_url");
        }
    }

    #[test]
    fn test_fetch_permissions_check_net_url_ipv4_loopback() {
        let mut permissions = Permissions {};
        let url = Url::parse("http://127.0.0.1/").unwrap();
        let result = permissions.check_net_url(&url, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_fetch_permissions_check_net_url_ipv6_loopback() {
        let mut permissions = Permissions {};
        let url = Url::parse("http://[::1]/").unwrap();
        let result = permissions.check_net_url(&url, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_fetch_permissions_check_open() {
        let mut permissions = Permissions {};
        let path = Cow::Borrowed(Path::new("/test/path"));
        let result = deno_fetch::FetchPermissions::check_open(
            &mut permissions,
            path,
            OpenAccessKind::Read,
            TEST_API_NAME,
        );

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "fetch_open");
        }
    }

    #[test]
    fn test_fetch_permissions_check_net_vsock() {
        let mut permissions = Permissions {};
        let result = permissions.check_net_vsock(1, 80, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "fetch_net_vsock");
        }
    }

    #[test]
    fn test_net_permissions_check_net() {
        let mut permissions = Permissions {};
        let host = ("example.com", Some(80));
        let result = permissions.check_net(&host, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net");
        }
    }

    #[test]
    fn test_net_permissions_check_open() {
        let mut permissions = Permissions {};
        let path = Cow::Borrowed(Path::new("/test/path"));
        let result = deno_net::NetPermissions::check_open(
            &mut permissions,
            path,
            OpenAccessKind::Write,
            TEST_API_NAME,
        );

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_open");
        }
    }

    #[test]
    fn test_net_permissions_check_vsock() {
        let mut permissions = Permissions {};
        let result = permissions.check_vsock(1, 80, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_vsock");
        }
    }

    #[test]
    fn test_check_addr_ipv4_public() {
        let addr = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let result = check_addr(addr, TEST_API_NAME);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_addr_ipv4_loopback() {
        let addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv4_unspecified() {
        let addr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv4_private() {
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv4_link_local() {
        let addr = IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1));
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv4_broadcast() {
        let addr = IpAddr::V4(Ipv4Addr::BROADCAST);
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv6_public() {
        let addr = IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888));
        let result = check_addr(addr, TEST_API_NAME);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_addr_ipv6_loopback() {
        let addr = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv6_unspecified() {
        let addr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv6_unique_local() {
        let addr = IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1));
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }

    #[test]
    fn test_check_addr_ipv6_link_local() {
        let addr = IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
        let result = check_addr(addr, TEST_API_NAME);

        assert!(result.is_err());
        if let Err(PermissionCheckError::PermissionDenied(err)) = result {
            assert_eq!(err.access, TEST_API_NAME);
            assert_eq!(err.name, "net_addr");
        }
    }
}
