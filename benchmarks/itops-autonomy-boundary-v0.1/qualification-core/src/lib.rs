//! Pure, dependency-free policy predicates for the synthetic ITOPS
//! autonomy-boundary benchmark.
//!
//! This crate deliberately contains only the strict attenuation and
//! root-reanchored policy arms. It does not issue or sign capabilities,
//! authenticate workload identity, establish fresh authority observations,
//! enforce a live participant commit boundary, or provide an independent
//! service observer. Those are separate integration and qualification layers.

#![forbid(unsafe_code)]

pub mod delegation;
pub mod effect_finality;
pub mod root_reanchor;
