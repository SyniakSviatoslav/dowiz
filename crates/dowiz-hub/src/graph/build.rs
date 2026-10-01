//! THE FOLD: the venue graph built from the order log and the catalogue --
//! orders, dishes, categories, ingredients, and who placed and delivered them.

use super::*;

impl Graph {
    /// THE FOLD. Reads the order log and the catalogue and relates them.
    ///
    /// Orders come from `hub.orders()`, which is already the newest state of
    /// each — so a dish that was removed from an order before it was cooked is
    /// not in the graph, because it is not in the order.
    pub fn of(hub: &Hub, catalog: &Catalog) -> Self {
        Self::of_with(hub, catalog, &HashMap::new())
    }

    /// The same fold, with LABELS THIS CRATE CANNOT KNOW.
    ///
    /// A courier's name is not in the hub. On the Worker the roster lives in a
    /// table this crate has never heard of, so a courier node could only be
    /// labelled with its own id — and a search for "Eni" found nothing, which
    /// is exactly the failure worth fixing: the graph knew the courier existed
    /// and could not say who they were.
    ///
    /// PASSED IN RATHER THAN FETCHED, because this crate has two dependencies
    /// and no network. The caller knows where its people live; this knows how
    /// they relate. Keys are node ids — `courier:<id>`, `ingredient:<id>` — so
    /// the same seam labels a shelf with its level or a customer with nothing,
    /// which is what a customer must stay labelled with.
    pub fn of_with(
        hub: &Hub,
        catalog: &Catalog,
        labels: &HashMap<String, String>,
    ) -> Self {
        let mut nodes: Vec<Node> = Vec::new();
        let mut edges: Vec<(usize, Rel, usize)> = Vec::new();
        let mut idx: HashMap<String, usize> = HashMap::new();

        let mut ensure = |nodes: &mut Vec<Node>,
                          idx: &mut HashMap<String, usize>,
                          id: String,
                          kind: Kind,
                          label: String,
                          text: String,
                          at_ms: i64| -> usize {
            if let Some(&i) = idx.get(&id) {
                // A node met twice keeps the richer description: an order names
                // a dish by id only, the catalogue names it properly, and which
                // arrives first is an accident of the fold order.
                if nodes[i].label.is_empty() && !label.is_empty() {
                    nodes[i].label = label;
                }
                if nodes[i].text.len() < text.len() {
                    nodes[i].text = text;
                }
                return i;
            }
            let i = nodes.len();
            idx.insert(id.clone(), i);
            nodes.push(Node { id, kind, label, text, at_ms });
            i
        };

        // ── the venue ──
        let venue_i = if let Some(loc) = catalog.location() {
            let name = str_field(&loc, "name").unwrap_or_default();
            let addr = str_field(&loc, "address").unwrap_or_default();
            let id = str_field(&loc, "id").unwrap_or_else(|| "venue".to_string());
            Some(ensure(
                &mut nodes,
                &mut idx,
                format!("venue:{id}"),
                Kind::Venue,
                name.clone(),
                format!("{name} {addr}"),
                0,
            ))
        } else {
            None
        };

        // ── dishes and their categories ──
        for (pid, pjson) in catalog.products() {
            let name = str_field(&pjson, "name").unwrap_or_default();
            let desc = str_field(&pjson, "description").unwrap_or_default();
            let price = int_field(&pjson, "price").unwrap_or(0);
            let dish = ensure(
                &mut nodes,
                &mut idx,
                format!("dish:{pid}"),
                Kind::Dish,
                name.clone(),
                format!("{name} {desc}"),
                0,
            );
            if let Some(v) = venue_i {
                edges.push((dish, Rel::ServedBy, v));
            }
            if let Some(cat) = str_field(&pjson, "categoryId").or_else(|| str_field(&pjson, "category")) {
                if !cat.is_empty() {
                    let c = ensure(
                        &mut nodes,
                        &mut idx,
                        format!("category:{cat}"),
                        Kind::Category,
                        cat.clone(),
                        cat.clone(),
                        0,
                    );
                    edges.push((dish, Rel::InCategory, c));
                }
            }
            // An ingredient the dish consumes, where the recipe says so. Both
            // spellings are accepted because the importer and the stock pane
            // disagree, and a graph that silently drops one would report a
            // shelf as unused.
            for item in objects_in(&pjson, "recipe")
                .into_iter()
                .chain(objects_in(&pjson, "ingredients"))
            {
                let Some(sid) = str_field(&item, "supplyId").or_else(|| str_field(&item, "id")) else {
                    continue;
                };
                if sid.is_empty() {
                    continue;
                }
                let s = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("ingredient:{sid}"),
                    Kind::Ingredient,
                    sid.clone(),
                    sid.clone(),
                    0,
                );
                edges.push((dish, Rel::Uses, s));
            }
            let _ = price;
        }

        // ── ingredients the shelf knows about, dish or no dish ──
        for (sid, sjson) in catalog.supplies() {
            let name = str_field(&sjson, "name").unwrap_or_else(|| sid.clone());
            let unit = str_field(&sjson, "unit").unwrap_or_default();
            ensure(
                &mut nodes,
                &mut idx,
                format!("ingredient:{sid}"),
                Kind::Ingredient,
                name.clone(),
                format!("{name} {unit}"),
                0,
            );
        }

        // ── orders, and everyone they touch ──
        for ev in hub.orders() {
            let j = &ev.order_json;
            let status = str_field(j, "status").unwrap_or_default();
            let total = int_field(j, "total").unwrap_or(0);
            let at = int_field(j, "created_at_ms").unwrap_or(ev.seq as i64);
            let order = ensure(
                &mut nodes,
                &mut idx,
                format!("order:{}", ev.order_id),
                Kind::Order,
                ev.order_id.clone(),
                format!("{} {status}", ev.order_id),
                at,
            );
            let _ = total;

            for item in objects_in(j, "items") {
                let Some(pid) = str_field(&item, "product_id")
                    .or_else(|| str_field(&item, "productId"))
                    .or_else(|| str_field(&item, "id"))
                else {
                    continue;
                };
                if pid.is_empty() {
                    continue;
                }
                let dish = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("dish:{pid}"),
                    Kind::Dish,
                    str_field(&item, "name").unwrap_or_default(),
                    str_field(&item, "name").unwrap_or_default(),
                    0,
                );
                edges.push((order, Rel::Contains, dish));
            }

            if let Some(cid) = str_field(j, "courier_id").filter(|s| !s.is_empty()) {
                let c = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("courier:{cid}"),
                    Kind::Courier,
                    cid.clone(),
                    cid.clone(),
                    0,
                );
                edges.push((order, Rel::DeliveredBy, c));
            }
            // THE CUSTOMER NODE CARRIES NO PII — see `Node::text`. The key is
            // already the pseudonym the customer list is built on.
            if let Some(key) = str_field(j, "customer_key")
                .or_else(|| str_field(j, "customer_id"))
                .filter(|s| !s.is_empty())
            {
                let k = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("customer:{key}"),
                    Kind::Customer,
                    String::new(),
                    String::new(),
                    0,
                );
                edges.push((order, Rel::PlacedBy, k));
            }
            if let Some(v) = venue_i {
                edges.push((order, Rel::ServedBy, v));
            }
        }

        // The caller's labels, last, so they win over an id used as a
        // placeholder. A label for a node that does not exist is ignored
        // rather than inventing one: the graph describes what happened, and
        // a courier who has carried nothing is not in it.
        for (id, label) in labels {
            if let Some(&i) = idx.get(id) {
                if !label.is_empty() {
                    nodes[i].label = label.clone();
                    nodes[i].text = if nodes[i].text.is_empty() {
                        label.clone()
                    } else {
                        format!("{} {label}", nodes[i].text)
                    };
                }
            }
        }

        Graph::build(nodes, edges)
    }
}
