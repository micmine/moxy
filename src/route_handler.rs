//! route operations
use crate::configuration::{Route, RouteMethod};

/// Checks if there is an existing route based on the path and method
//pub fn get_route(routes: &Vec<Route>, path: &str, method: &RouteMethod) -> Option<&Route> {
//let matching_routes = routes
//.iter()
//.find(|c| c.path.as_str() == path && &c.method == method);

//matching_routes
//}

/// Checks if there is an existing route based on the resource and method
pub fn get_route_by_resource_mut<'a>(
    routes: &'a mut Vec<Route>,
    resource: &'a str,
    method: &'a RouteMethod,
) -> Option<&'a mut Route> {
    let matching_routes = routes
        .iter_mut()
        .filter(|c| c.resource.is_some())
        .find(|c| c.resource.as_ref().unwrap().as_str() == resource && &c.method == method);

    matching_routes
}

/// Checks if there is an existing route based on the path and method
pub fn get_route_by_path_mut<'a>(
    routes: &'a mut Vec<Route>,
    path: &'a str,
    method: &'a RouteMethod,
) -> Option<&'a mut Route> {
    let matching_routes = routes
        .iter_mut()
        .find(|c| c.path.as_str() == path && &c.method == method);

    matching_routes
}

/// Returns the route and an optional parameter.
///
/// The parameter can be used to milify the configuration when there is one dynamic part of the url
/// and file path.
///
/// | path   | file       |
/// |--------|------------|
/// | /a.txt | ./db/a.txt |
/// | /b.txt | ./db/b.txt |
/// | /c.txt | ./db/c.txt |
/// | /d.txt | ./db/d.txt |
/// | /e.txt | ./db/e.txt |
///
/// In order to ceate configuration for this there would be a configuration entry for every path.
/// But this can be simplified.
/// ``` json
/// {
///     "method": "GET",
///     "path": "/$$$.txt",
///     "resource": "./db/$$$.txt"
/// }
/// ```
pub fn get_route<'a>(
    routes: &'a Vec<Route>,
    path: &'a str,
    method: &'a RouteMethod,
) -> (Option<&'a Route>, Option<&'a str>) {
    for i in routes.iter() {
        if i.method.eq(&method) {
            let index = &i.path.find("$$$");

            if let Some(index) = index {
                let match_before = &i.path[0..*index];

                if path.starts_with(&match_before) {
                    if index + 3 != i.path.len() {
                        let match_end = &i.path[index + 3..i.path.len()];

                        if path.ends_with(match_end) {
                            let sd = match_end.len();
                            return (Some(i), Some(&path[i.path.len() - 3 - sd..path.len() - sd]));
                        }
                    } else {
                        return (Some(i), Some(&path[i.path.len() - 3..path.len()]));
                    }
                }
            }
            if path.ends_with(&i.path) {
                return (Some(i), None);
            }
        }
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::{Route, RouteMethod, WsMessageTime},
        route_handler::get_route,
    };

    #[test]
    fn static_route() {
        let routes = vec![Route {
            method: RouteMethod::GET,
            path: "/api/test".to_string(),
            resource: Some("db/api/test.json".to_string()),
            messages: vec![],
        }];
        let url = "http://localhost:8080/api/test";
        let (result, parameter) = get_route(&routes, &url, &RouteMethod::GET);

        assert_eq!(result.unwrap().resource, routes[0].resource);
        assert_eq!(parameter, None);
    }

    //#[test]
    //fn configuration_get_route_should_find_no_route() {
    //let routes = vec![
    //Route {
    //method: RouteMethod::GET,
    //path: "/a".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //Route {
    //method: RouteMethod::GET,
    //path: "/b".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //Route {
    //method: RouteMethod::GET,
    //path: "/c".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //];

    //assert!(!get_route(&routes, "/abc", RouteMethod::GET).is_some());
    //}

    //#[test]
    //fn configuration_get_route_should_find_route() {
    //let routes = vec![
    //Route {
    //method: RouteMethod::GET,
    //path: "/a".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //Route {
    //method: RouteMethod::GET,
    //path: "/b".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //Route {
    //method: RouteMethod::GET,
    //path: "/c".to_string(),
    //resource: Some("somefile.txt".to_string()),
    //messages: vec![],
    //},
    //];

    //assert!(get_route(&routes, "/a", &RouteMethod::GET).is_some());
    //assert!(get_route(&routes, "/b", &RouteMethod::GET).is_some());
    //assert!(get_route(&routes, "/c", &RouteMethod::GET).is_some());
    //}

    #[test]
    fn dynamic_route_with_different_start() {
        let routes = vec![
            Route {
                method: RouteMethod::GET,
                path: "/api/test/1/$$$.json".to_string(),
                resource: Some("db/api/1/$$$.json".to_string()),
                messages: vec![],
            },
            Route {
                method: RouteMethod::GET,
                path: "/api/test/2/$$$.json".to_string(),
                resource: Some("db/api/2/$$$.json".to_string()),
                messages: vec![],
            },
            Route {
                method: RouteMethod::GET,
                path: "/api/test/3/$$$.json".to_string(),
                resource: Some("db/api/3/$$$.json".to_string()),
                messages: vec![],
            },
        ];

        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/1/abc.json",
                &RouteMethod::GET
            )
            .0
            .unwrap()
            .resource
            .as_ref()
            .unwrap(),
            "db/api/1/$$$.json"
        );
        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/2/abc.json",
                &RouteMethod::GET
            )
            .0
            .unwrap()
            .resource
            .as_ref()
            .unwrap(),
            "db/api/2/$$$.json"
        );
        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/3/abc.json",
                &RouteMethod::GET
            )
            .0
            .unwrap()
            .resource
            .as_ref()
            .unwrap(),
            "db/api/3/$$$.json"
        );
    }

    #[test]
    fn dynamic_route_with_different_end() {
        let routes = vec![
            Route {
                method: RouteMethod::GET,
                path: "/api/test/$$$.txt".to_string(),
                resource: Some("db/api/$$$.txt".to_string()),
                messages: vec![],
            },
            Route {
                method: RouteMethod::GET,
                path: "/api/test/$$$.json".to_string(),
                resource: Some("db/api/$$$.json".to_string()),
                messages: vec![],
            },
        ];

        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/abc.txt",
                &RouteMethod::GET
            )
            .0
            .unwrap()
            .resource
            .as_ref()
            .unwrap(),
            "db/api/$$$.txt"
        );
        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/abc.json",
                &RouteMethod::GET
            )
            .0
            .unwrap()
            .resource
            .as_ref()
            .unwrap(),
            "db/api/$$$.json"
        );
    }

    #[test]
    fn dynamic_paramerter_end() {
        let routes = vec![Route {
            method: RouteMethod::GET,
            path: "/api/test/$$$".to_string(),
            resource: Some("db/api/$$$".to_string()),
            messages: vec![],
        }];

        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/abc",
                &RouteMethod::GET
            )
            .1
            .unwrap(),
            "abc"
        );
    }

    #[test]
    fn dynamic_paramerter_middle() {
        let routes = vec![Route {
            method: RouteMethod::GET,
            path: "/api/test/$$$.txt".to_string(),
            resource: Some("db/api/$$$.txt".to_string()),
            messages: vec![],
        }];

        assert_eq!(
            get_route(
                &routes,
                "http://localhost:8080/api/test/abc.txt",
                &RouteMethod::GET
            )
            .1
            .unwrap(),
            "abc"
        );
    }

    #[test]
    fn parse_ws_message_time() {
        assert_eq!(
            "3s".parse::<WsMessageTime>().unwrap(),
            WsMessageTime::Second(3)
        );
        assert_eq!(
            "3m".parse::<WsMessageTime>().unwrap(),
            WsMessageTime::Minute(3)
        );
        assert_eq!(
            "3h".parse::<WsMessageTime>().unwrap(),
            WsMessageTime::Hour(3)
        );
        assert_eq!(
            "3sent".parse::<WsMessageTime>().unwrap(),
            WsMessageTime::Sent(3)
        );
        assert_eq!(
            "3recived".parse::<WsMessageTime>().unwrap(),
            WsMessageTime::Recived(3)
        );
    }

    #[test]
    fn get_route_should_not_find_entry_if_the_url_only_partialy_matches() {
        let routes = vec![Route {
            method: RouteMethod::GET,
            path: "/a".to_string(),
            resource: Some("".to_string()),
            messages: vec![],
        }];

        let uri = "/a/test";

        let result = get_route(&routes, &uri, &RouteMethod::GET);

        assert_eq!(result, (None, None));
    }
}
