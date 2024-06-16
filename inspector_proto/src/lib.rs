include!(concat!(env!("OUT_DIR"), "/nodo.inspector.rs"));

// Copyright 2022 by David Weikersdorfer

use crate as nodi;
use std::collections::HashMap;
use url::Url;

pub fn is_valid(ws: Option<&nodi::Worldstate>) -> bool {
    ws.is_some() && ws.unwrap().manifold.is_some()
}

/// Extracts TAG for given A from scheme://path/to/foo?a=A&tag=TAG&something=else
fn uri_parse_tag(uri: &str, a: &str) -> Option<String> {
    let url = Url::parse(uri).ok()?;
    let mut found = false;
    let mut tag: Option<String> = None;
    for x in url.query()?.split('&') {
        let mut tokens = x.split('=');
        let lhs = tokens.next()?;
        let rhs = tokens.next()?;
        if lhs == "a" && rhs == a {
            found = true;
        }
        if lhs == "tag" {
            tag = Some(rhs.to_string());
        }
    }
    if found {
        tag
    } else {
        None
    }
}

pub fn uri_path_last(uri: &str) -> Option<String> {
    let url = Url::parse(uri).ok()?;
    Some(url.path_segments()?.last()?.to_string())
}

pub fn lifecycle_state_to_str(lifecycle_state: i32) -> &'static str {
    use nodi::LifecycleState::*;
    match nodi::LifecycleState::from_i32(lifecycle_state) {
        Some(Barren) => "Barren",
        Some(Inactive) => "Inactive",
        Some(Running) => "Running",
        Some(Paused) => "Paused",
        Some(Failed) => "Failed",
        Some(Invalid) | None => "(err)",
    }
}

impl nodi::Worldstate {
    pub fn uri_full(&self, uid: u64) -> Option<&str> {
        Some(&self.uris.get(&uid)?.fulltext)
    }

    pub fn uri_tag(&self, uid: u64, a: &str) -> Option<String> {
        uri_parse_tag(self.uri_full(uid)?, a)
    }

    pub fn uri_tag_or_none(&self, uid: u64, a: &str) -> String {
        self.uri_tag(uid, a)
            .unwrap_or_else(|| String::from("(none)"))
    }

    pub fn vertices_hsv(&self) -> Vec<(u64, &str, &nodi::Vertex)> {
        let mut v: Vec<_> = self
            .manifold
            .as_ref()
            .unwrap()
            .vertices
            .iter()
            .filter_map(|(&uid, v)| self.uri_full(uid).map(|s| (uid, s, v)))
            .collect();
        v.sort_by_key(|&(_, s, _)| s); // FIXME why clone?
        v
    }

    pub fn find_vertex_by_uid(&self, uid: u64) -> Option<&nodi::Vertex> {
        self.manifold.as_ref().unwrap().vertices.get(&uid)
    }

    fn as_uid_name_value<'a, T>(
        &self,
        attachments: &'a HashMap<u64, T>,
        tag: &str,
    ) -> Vec<(u64, String, &'a T)> {
        let mut v: Vec<_> = attachments
            .iter()
            .filter_map(|(&uid, v)| self.uri_tag(uid, tag).map(|name| (uid, name, v)))
            .collect();
        v.sort_by_key(|(_, s, _)| s.clone()); // FIXME why clone?
        v
    }

    pub fn vertex_rx_channels<'a>(
        &self,
        vertex: &'a nodi::Vertex,
    ) -> Vec<(u64, String, &'a nodi::Channel)> {
        self.as_uid_name_value(&vertex.rx_channels, "rx")
    }

    pub fn vertex_tx_channels<'a>(
        &self,
        vertex: &'a nodi::Vertex,
    ) -> Vec<(u64, String, &'a nodi::Channel)> {
        self.as_uid_name_value(&vertex.tx_channels, "tx")
    }

    pub fn vertex_parameters<'a>(
        &self,
        vertex: &'a nodi::Vertex,
    ) -> Vec<(u64, String, &'a nodi::Parameter)> {
        self.as_uid_name_value(&vertex.parameters, "p")
    }

    pub fn vertex_conditions_individual<'a>(
        &self,
        vertex: &'a nodi::Vertex,
    ) -> Vec<(u64, String, &'a nodi::ConditionResult)> {
        if let Some(exec) = vertex.execution_data.as_ref() {
            if let Some(cstate) = exec.conditions_state.as_ref() {
                return self.as_uid_name_value(&cstate.individual, "condition");
            }
        }
        vec![]
    }

    pub fn vertex_parameter_by_uri<'a>(
        &self,
        vertex: &'a nodi::Vertex,
        uri: u64,
    ) -> Option<&'a nodi::Parameter> {
        vertex.parameters.get(&uri)
    }

    pub fn find_parameter_by_uri(&self, uri: &str) -> Option<&nodi::Vertex> {
        let uid = self.uris.iter().find_map(
            |(key, val)| {
                if val.fulltext == uri {
                    Some(key)
                } else {
                    None
                }
            },
        );
        self.manifold.as_ref().unwrap().vertices.get(uid?)
    }

    pub fn num_vertices(&self) -> usize {
        self.manifold().vertices.len()
    }

    pub fn manifold(&self) -> &nodi::Manifold {
        self.manifold.as_ref().unwrap()
    }
}
