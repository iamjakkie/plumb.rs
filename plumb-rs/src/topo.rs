use std::collections::{HashMap, HashSet, VecDeque};

use crate::engine::{EngineError, SpecEdge, SpecNode};

/// Kahn topological sort. Returns node ids in processing order (sources first).
pub fn topo_sort(nodes: &[SpecNode], edges: &[SpecEdge]) -> Result<Vec<i32>, EngineError> {
    let ids: HashSet<i32> = nodes.iter().map(|n| n.id).collect();
    for e in edges {
        if !ids.contains(&e.from) || !ids.contains(&e.to) {
            return Err(EngineError::InvalidGraph(format!(
                "edge {}→{} references unknown node",
                e.from, e.to
            )));
        }
    }

    let mut indegree: HashMap<i32, usize> = ids.iter().map(|&id| (id, 0)).collect();
    let mut outgoing: HashMap<i32, Vec<i32>> = HashMap::new();
    for e in edges {
        *indegree.entry(e.to).or_default() += 1;
        outgoing.entry(e.from).or_default().push(e.to);
    }

    let mut queue: VecDeque<i32> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.make_contiguous().sort();

    let mut order = Vec::with_capacity(nodes.len());
    while let Some(id) = queue.pop_front() {
        order.push(id);
        if let Some(nexts) = outgoing.get(&id) {
            for &n in nexts {
                let d = indegree.get_mut(&n).unwrap();
                *d -= 1;
                if *d == 0 {
                    queue.push_back(n);
                }
            }
        }
    }

    if order.len() != nodes.len() {
        return Err(EngineError::InvalidGraph(
            "pipeline graph contains a cycle".into(),
        ));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SpecNode;
    use serde_json::json;

    fn node(id: i32, kind: &str) -> SpecNode {
        SpecNode {
            id,
            kind: kind.into(),
            name: format!("n{id}"),
            config: json!({}),
        }
    }

    #[test]
    fn sorts_linear() {
        let nodes = vec![node(1, "connector"), node(2, "transformation"), node(3, "destination")];
        let edges = vec![
            SpecEdge { from: 1, to: 2 },
            SpecEdge { from: 2, to: 3 },
        ];
        assert_eq!(topo_sort(&nodes, &edges).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn rejects_cycle() {
        let nodes = vec![node(1, "connector"), node(2, "transformation")];
        let edges = vec![
            SpecEdge { from: 1, to: 2 },
            SpecEdge { from: 2, to: 1 },
        ];
        assert!(topo_sort(&nodes, &edges).is_err());
    }
}
