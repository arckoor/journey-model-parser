use std::collections::HashMap;

use serde::Serialize;

use crate::{
    error::ParseError,
    structure::{Node, NodeKind, Transform},
    Options,
};

#[derive(Serialize, Debug)]
pub struct Armature {
    pub root: Joint,
}

#[derive(Serialize, Debug)]
pub struct Joint {
    pub name: String,
    pub children: Vec<Joint>,
    pub transform: Transform,
    pub aliases: Vec<String>,
}

impl Armature {
    pub fn new(options: &Options, root_node: &NodeKind) -> Result<Option<Self>, ParseError> {
        let transform = if let NodeKind::Root(root_node) = root_node {
            root_node.transform.clone()
        } else {
            return Err(ParseError::new("Expected root node"));
        };

        let mut children = Self::flatten(root_node, Vec::new(), options.armature_full);
        if children.is_empty() {
            return Ok(None);
        }

        if children.len() == 1 {
            let mut root = children.remove(0);
            // Self::prune_aliases(&mut root);
            return Ok(Some(Self { root }));
        }

        let mut root = Joint {
            name: "root".to_string(),
            children,
            transform,
            aliases: Vec::new(),
        };

        // probably unnecessary, delete when sure
        // Self::prune_aliases(&mut root);

        Ok(Some(Self { root }))
    }

    fn flatten(node: &NodeKind, mut aliases: Vec<String>, full: bool) -> Vec<Joint> {
        let mut joint_nodes = Vec::new();

        let mut process_joint = |node: &Node| {
            let children = node
                .nodes
                .iter()
                .flat_map(|child| Self::flatten(child, Vec::new(), full))
                .collect();
            joint_nodes.push(Joint {
                name: node.id.clone(),
                children,
                transform: node.transform.clone(),
                aliases: aliases.clone(),
            });
        };

        match node {
            NodeKind::Joint(joint) => {
                process_joint(joint);
            }
            NodeKind::Root(node) | NodeKind::Render(node) | NodeKind::Node(node) => {
                if full {
                    process_joint(node);
                } else {
                    aliases.push(node.id.clone());
                    joint_nodes.extend(
                        node.nodes
                            .iter()
                            .flat_map(|child| Self::flatten(child, aliases.clone(), full)),
                    );
                }
            }
        }

        joint_nodes
    }

    fn prune_aliases(node: &mut Joint) {
        let mut common = HashMap::new();
        for child in &node.children {
            for alias in &child.aliases {
                let count = common.entry(alias.clone()).or_insert(0);
                *count += 1;
            }
        }
        for child in &mut node.children {
            for alias in child.aliases.clone() {
                if *common.get(&alias).unwrap() > 1 {
                    child.aliases.retain(|a| a != &alias);
                }
            }
        }

        for child in &mut node.children {
            Self::prune_aliases(child);
        }
    }
}
