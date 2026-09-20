//! Projections: what a reader needs, folded once, rather than the whole image.
//!
//! Every reader — the console's queue, the courier's runs, the customer's one
//! order — used to fetch the venue's entire log out of the Durable Object and
//! fold it in the Worker, so a poll cost O(image) on the wire and O(events) of
//! CPU, both growing with how long the venue had been open. The fold belongs
//! where the image already lives, and its answer is the same for every reader
//! until the next append: one fold per generation, not one per poll.
//!
//! The functions here are PURE so they can be tested natively; the Durable
//! Object in `hubdo` calls them and memoises by generation.
