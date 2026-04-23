use crate::models::petri_net::{Arc, PetriNet, Place, Transition};
use crate::utils::dense_kernel::fnv1a_64;
use anyhow::{anyhow, Result};
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;

pub fn read_pnml(path: &Path) -> Result<PetriNet> {
    let content = std::fs::read(path)
        .map_err(|e| anyhow!("failed to read {}: {}", path.display(), e))?;
    parse_pnml(&content)
}

pub fn parse_pnml(content: &[u8]) -> Result<PetriNet> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);

    struct PlaceInfo {
        xml_id: String,
        label: String,
        initial_count: usize,
    }

    struct TransInfo {
        xml_id: String,
        label: String,
        local_node_id: String,
        is_invisible: bool,
    }

    struct ArcInfo {
        source: String,
        target: String,
    }

    let mut places: Vec<PlaceInfo> = Vec::new();
    let mut transitions: Vec<TransInfo> = Vec::new();
    let mut arc_infos: Vec<ArcInfo> = Vec::new();
    let mut final_place_counts: Vec<(String, usize)> = Vec::new();

    #[derive(Clone, Copy, PartialEq)]
    enum Context {
        Root,
        InPlace,
        InTransition,
        InArc,
        InFinalMarkings,
        InFMPlace,
    }

    #[derive(Clone, Copy, PartialEq)]
    enum TextTarget {
        None,
        Label,
        InitialCount,
        FinalCount,
    }

    let mut ctx = Context::Root;
    let mut text_target = TextTarget::None;
    let mut in_name = false;
    let mut in_initial_marking = false;

    let mut cur_xml_id = String::new();
    let mut cur_label = String::new();
    let mut cur_local_id = String::new();
    let mut cur_activity = String::new();
    let mut cur_initial: usize = 0;
    let mut cur_arc_src = String::new();
    let mut cur_arc_tgt = String::new();
    let mut cur_fm_idref = String::new();
    let mut cur_fm_count: usize = 0;

    let mut buf = Vec::with_capacity(2048);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => {
                let tag = e.name().as_ref().to_vec();
                match tag.as_slice() {
                    b"place" => {
                        let mut id_attr = String::new();
                        let mut idref_attr = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => id_attr = String::from_utf8_lossy(&attr.value).into_owned(),
                                b"idref" => idref_attr = String::from_utf8_lossy(&attr.value).into_owned(),
                                _ => {}
                            }
                        }
                        if ctx == Context::InFinalMarkings && !idref_attr.is_empty() {
                            ctx = Context::InFMPlace;
                            cur_fm_idref = idref_attr;
                            cur_fm_count = 0;
                        } else if !id_attr.is_empty() {
                            ctx = Context::InPlace;
                            cur_xml_id = id_attr;
                            cur_label.clear();
                            cur_initial = 0;
                        }
                    }
                    b"transition" => {
                        cur_xml_id.clear();
                        cur_label.clear();
                        cur_local_id.clear();
                        cur_activity.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                cur_xml_id = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                        }
                        ctx = Context::InTransition;
                    }
                    b"arc" => {
                        cur_arc_src.clear();
                        cur_arc_tgt.clear();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"source" => cur_arc_src = String::from_utf8_lossy(&attr.value).into_owned(),
                                b"target" => cur_arc_tgt = String::from_utf8_lossy(&attr.value).into_owned(),
                                _ => {}
                            }
                        }
                        ctx = Context::InArc;
                    }
                    b"name" if ctx == Context::InPlace || ctx == Context::InTransition => {
                        in_name = true;
                    }
                    b"text" if in_name && (ctx == Context::InPlace || ctx == Context::InTransition) => {
                        text_target = TextTarget::Label;
                    }
                    b"text" if in_initial_marking && ctx == Context::InPlace => {
                        text_target = TextTarget::InitialCount;
                    }
                    b"text" if ctx == Context::InFMPlace => {
                        text_target = TextTarget::FinalCount;
                    }
                    b"initialMarking" if ctx == Context::InPlace => {
                        in_initial_marking = true;
                    }
                    b"finalmarkings" => {
                        ctx = Context::InFinalMarkings;
                    }
                    _ => {}
                }
            }

            Ok(XmlEvent::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                match tag.as_slice() {
                    b"toolspecific" if ctx == Context::InTransition => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"localNodeID" => cur_local_id = String::from_utf8_lossy(&attr.value).into_owned(),
                                b"activity" => cur_activity = String::from_utf8_lossy(&attr.value).into_owned(),
                                _ => {}
                            }
                        }
                    }
                    b"place" if ctx == Context::InFinalMarkings => {
                        // <place idref="..."/> inside finalmarkings (self-closing, count=0)
                        let mut idref = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"idref" {
                                idref = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                        }
                        if !idref.is_empty() {
                            final_place_counts.push((idref, 0));
                        }
                    }
                    _ => {}
                }
            }

            Ok(XmlEvent::Text(t)) if text_target != TextTarget::None => {
                let text = t.unescape().unwrap_or_default();
                let text = text.trim();
                match text_target {
                    TextTarget::Label => cur_label = text.to_string(),
                    TextTarget::InitialCount => cur_initial = text.parse().unwrap_or(0),
                    TextTarget::FinalCount => cur_fm_count = text.parse().unwrap_or(0),
                    TextTarget::None => {}
                }
                text_target = TextTarget::None;
            }

            Ok(XmlEvent::End(e)) => {
                let tag = e.name().as_ref().to_vec();
                match tag.as_slice() {
                    b"place" => {
                        match ctx {
                            Context::InPlace => {
                                places.push(PlaceInfo {
                                    xml_id: cur_xml_id.clone(),
                                    label: cur_label.clone(),
                                    initial_count: cur_initial,
                                });
                                ctx = Context::Root;
                                in_name = false;
                                in_initial_marking = false;
                            }
                            Context::InFMPlace => {
                                final_place_counts.push((cur_fm_idref.clone(), cur_fm_count));
                                ctx = Context::InFinalMarkings;
                            }
                            _ => {}
                        }
                    }
                    b"transition" => {
                        let effective_label = if !cur_activity.is_empty() {
                            cur_activity.clone()
                        } else {
                            cur_label.clone()
                        };
                        let invisible = effective_label == "$invisible$";
                        transitions.push(TransInfo {
                            xml_id: cur_xml_id.clone(),
                            label: effective_label,
                            local_node_id: if cur_local_id.is_empty() {
                                cur_xml_id.clone()
                            } else {
                                cur_local_id.clone()
                            },
                            is_invisible: invisible,
                        });
                        ctx = Context::Root;
                        in_name = false;
                    }
                    b"arc" => {
                        if !cur_arc_src.is_empty() && !cur_arc_tgt.is_empty() {
                            arc_infos.push(ArcInfo {
                                source: cur_arc_src.clone(),
                                target: cur_arc_tgt.clone(),
                            });
                        }
                        ctx = Context::Root;
                    }
                    b"name" => {
                        in_name = false;
                    }
                    b"initialMarking" => {
                        in_initial_marking = false;
                    }
                    b"finalmarkings" => {
                        ctx = Context::Root;
                    }
                    _ => {}
                }
            }

            Ok(XmlEvent::Eof) => break,
            Err(e) => return Err(anyhow!("PNML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // Build xml_id → unique_id translation map
    let mut xml_to_unique: HashMap<String, String> = HashMap::new();
    for p in &places {
        xml_to_unique.insert(p.xml_id.clone(), p.label.clone());
    }
    for t in &transitions {
        xml_to_unique.insert(t.xml_id.clone(), t.local_node_id.clone());
    }

    let mut net = PetriNet::default();

    for p in &places {
        net.places.push(Place { id: p.label.clone() });
        if p.initial_count > 0 {
            let h = fnv1a_64(p.label.as_bytes());
            net.initial_marking.insert(h, p.label.clone(), p.initial_count);
        }
    }

    for t in &transitions {
        net.transitions.push(Transition {
            id: t.local_node_id.clone(),
            label: t.label.clone(),
            is_invisible: Some(t.is_invisible),
        });
    }

    for a in &arc_infos {
        let from = xml_to_unique
            .get(&a.source)
            .ok_or_else(|| anyhow!("arc source {} not found", a.source))?
            .clone();
        let to = xml_to_unique
            .get(&a.target)
            .ok_or_else(|| anyhow!("arc target {} not found", a.target))?
            .clone();
        net.arcs.push(Arc { from, to, weight: Some(1) });
    }

    // Build final marking from the finalmarkings section
    if !final_place_counts.is_empty() {
        let mut fm: crate::utils::dense_kernel::PackedKeyTable<String, usize> =
            crate::utils::dense_kernel::PackedKeyTable::default();
        for (xml_id, count) in &final_place_counts {
            if let Some(label) = xml_to_unique.get(xml_id) {
                let h = fnv1a_64(label.as_bytes());
                fm.insert(h, label.clone(), *count);
            }
        }
        net.final_markings.push(fm);
    }

    net.compile_incidence();
    Ok(net)
}
