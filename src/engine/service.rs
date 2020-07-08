use crate::domain::{Target, TargetId};
use std::collections::{HashMap, HashSet};

/// List targets of the service graph.
///
/// Returns the target IDs of the services, omitting those that are only required by build targets.
pub fn get_service_graph_targets(
    targets: &HashMap<TargetId, Target>,
    root_target_ids: &[TargetId],
) -> HashSet<TargetId> {
    root_target_ids
        .iter()
        .fold(HashSet::new(), |mut service_ids, target_id| {
            let target = targets.get(&target_id).unwrap();

            match target {
                Target::Service(_) => {
                    service_ids.insert(target_id.clone());
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, target.dependencies()))
                        .cloned()
                        .collect();
                }
                Target::Aggregate(_) => {
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, target.dependencies()))
                        .cloned()
                        .collect();
                }
                Target::Build(_) => {}
            }

            service_ids
        })
}
