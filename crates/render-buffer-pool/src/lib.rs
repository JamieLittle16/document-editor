#![doc = "Bounded host-owned render-buffer admission and lease lifecycle."]

use std::collections::BTreeMap;
use std::fmt;

/// Stable host-owned identifier for one reusable render-buffer slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderBufferId(u32);

impl RenderBufferId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Monotonic reuse generation for one render-buffer slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderLeaseGeneration(u64);

impl RenderLeaseGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identity required to complete or release one specific slot lease.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderLeaseId {
    buffer_id: RenderBufferId,
    generation: RenderLeaseGeneration,
}

impl RenderLeaseId {
    #[must_use]
    pub const fn buffer_id(self) -> RenderBufferId {
        self.buffer_id
    }

    #[must_use]
    pub const fn generation(self) -> RenderLeaseGeneration {
        self.generation
    }
}

/// Hard host-owned admission limits for one render-buffer pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderBufferPoolLimits {
    max_total_bytes: usize,
    max_slots: usize,
    max_slot_bytes: usize,
    max_outstanding_leases_per_scope: usize,
}

impl RenderBufferPoolLimits {
    pub fn new(
        max_total_bytes: usize,
        max_slots: usize,
        max_slot_bytes: usize,
        max_outstanding_leases_per_scope: usize,
    ) -> Result<Self, RenderBufferPoolLimitsError> {
        if max_total_bytes == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroTotalBytes);
        }
        if max_slots == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroSlots);
        }
        if u32::try_from(max_slots).is_err() {
            return Err(RenderBufferPoolLimitsError::TooManySlots);
        }
        if max_slot_bytes == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroSlotBytes);
        }
        if max_slot_bytes > max_total_bytes {
            return Err(RenderBufferPoolLimitsError::SlotExceedsTotalBudget);
        }
        if max_outstanding_leases_per_scope == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroOutstandingLeasesPerScope);
        }
        Ok(Self {
            max_total_bytes,
            max_slots,
            max_slot_bytes,
            max_outstanding_leases_per_scope,
        })
    }

    #[must_use]
    pub const fn max_total_bytes(self) -> usize {
        self.max_total_bytes
    }

    #[must_use]
    pub const fn max_slots(self) -> usize {
        self.max_slots
    }

    #[must_use]
    pub const fn max_slot_bytes(self) -> usize {
        self.max_slot_bytes
    }

    #[must_use]
    pub const fn max_outstanding_leases_per_scope(self) -> usize {
        self.max_outstanding_leases_per_scope
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferPoolLimitsError {
    ZeroTotalBytes,
    ZeroSlots,
    TooManySlots,
    ZeroSlotBytes,
    SlotExceedsTotalBudget,
    ZeroOutstandingLeasesPerScope,
}

impl fmt::Display for RenderBufferPoolLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTotalBytes => formatter.write_str("render pool total byte budget must be nonzero"),
            Self::ZeroSlots => formatter.write_str("render pool slot budget must be nonzero"),
            Self::TooManySlots => formatter.write_str("render pool slot budget exceeds u32 identity space"),
            Self::ZeroSlotBytes => formatter.write_str("render pool maximum slot capacity must be nonzero"),
            Self::SlotExceedsTotalBudget => {
                formatter.write_str("render pool maximum slot capacity exceeds total byte budget")
            }
            Self::ZeroOutstandingLeasesPerScope => formatter
                .write_str("render pool per-scope outstanding lease budget must be nonzero"),
        }
    }
}

impl std::error::Error for RenderBufferPoolLimitsError {}

/// Public lifecycle state without exposing a caller's authority/scope value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferSlotState {
    Available,
    Leased,
    Ready,
    Retained,
}

/// Host-issued write authority for one buffer slot generation.
///
/// Cloning this value does not extend the underlying lease lifetime. A stale clone is rejected
/// after recycle/reuse by `RenderLeaseGeneration`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderWriteLease<Scope> {
    id: RenderLeaseId,
    scope: Scope,
    capacity_bytes: usize,
    requested_bytes: usize,
}

impl<Scope> RenderWriteLease<Scope> {
    #[must_use]
    pub const fn id(&self) -> RenderLeaseId {
        self.id
    }

    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    #[must_use]
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn requested_bytes(&self) -> usize {
        self.requested_bytes
    }
}

/// Host-visible publication token for bytes completed under one valid write lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadyRenderBuffer<Scope> {
    id: RenderLeaseId,
    scope: Scope,
    capacity_bytes: usize,
    written_bytes: usize,
}

impl<Scope> ReadyRenderBuffer<Scope> {
    #[must_use]
    pub const fn id(&self) -> RenderLeaseId {
        self.id
    }

    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    #[must_use]
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn written_bytes(&self) -> usize {
        self.written_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferAcquireError {
    ZeroByteRequest,
    RequestExceedsSlotLimit { requested: usize, limit: usize },
    ScopeLeaseLimitReached { limit: usize },
    PoolExhausted,
    LeaseGenerationExhausted { buffer_id: RenderBufferId },
}

impl fmt::Display for RenderBufferAcquireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroByteRequest => formatter.write_str("render lease request must be nonzero"),
            Self::RequestExceedsSlotLimit { requested, limit } => write!(
                formatter,
                "render lease requests {requested} bytes, exceeding slot limit {limit}"
            ),
            Self::ScopeLeaseLimitReached { limit } => write!(
                formatter,
                "render scope already owns the maximum {limit} outstanding write leases"
            ),
            Self::PoolExhausted => formatter.write_str("render buffer pool is exhausted"),
            Self::LeaseGenerationExhausted { buffer_id } => write!(
                formatter,
                "render buffer {} exhausted its lease generation space",
                buffer_id.get()
            ),
        }
    }
}

impl std::error::Error for RenderBufferAcquireError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferTransitionError {
    UnknownBuffer(RenderBufferId),
    StaleLease {
        buffer_id: RenderBufferId,
        expected_generation: RenderLeaseGeneration,
        actual_generation: RenderLeaseGeneration,
    },
    WrongState {
        expected: RenderBufferSlotState,
        actual: RenderBufferSlotState,
    },
    WrongScope,
    ZeroByteCompletion,
    CompletionExceedsRequest {
        written: usize,
        requested: usize,
    },
}

impl fmt::Display for RenderBufferTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBuffer(buffer_id) => {
                write!(formatter, "unknown render buffer {}", buffer_id.get())
            }
            Self::StaleLease {
                buffer_id,
                expected_generation,
                actual_generation,
            } => write!(
                formatter,
                "stale render lease for buffer {}: expected generation {}, got {}",
                buffer_id.get(),
                expected_generation.get(),
                actual_generation.get()
            ),
            Self::WrongState { expected, actual } => write!(
                formatter,
                "render buffer is in state {actual:?}, expected {expected:?}"
            ),
            Self::WrongScope => formatter.write_str("render buffer lease belongs to another scope"),
            Self::ZeroByteCompletion => {
                formatter.write_str("render buffer completion must publish at least one byte")
            }
            Self::CompletionExceedsRequest { written, requested } => write!(
                formatter,
                "render completion publishes {written} bytes, exceeding requested {requested} bytes"
            ),
        }
    }
}

impl std::error::Error for RenderBufferTransitionError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderBufferPoolStats {
    pub total_capacity_bytes: usize,
    pub slot_count: usize,
    pub available_slots: usize,
    pub leased_slots: usize,
    pub ready_slots: usize,
    pub retained_slots: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderScopeInvalidation {
    pub leased_reclaimed: usize,
    pub ready_reclaimed: usize,
    pub retained_reclaimed: usize,
}

impl RenderScopeInvalidation {
    #[must_use]
    pub const fn reclaimed_slots(self) -> usize {
        self.leased_reclaimed + self.ready_reclaimed + self.retained_reclaimed
    }
}

#[derive(Debug)]
enum SlotState<Scope> {
    Available,
    Leased {
        scope: Scope,
        requested_bytes: usize,
    },
    Ready {
        scope: Scope,
        written_bytes: usize,
    },
    Retained {
        scope: Scope,
        written_bytes: usize,
    },
}

impl<Scope> SlotState<Scope> {
    const fn public_state(&self) -> RenderBufferSlotState {
        match self {
            Self::Available => RenderBufferSlotState::Available,
            Self::Leased { .. } => RenderBufferSlotState::Leased,
            Self::Ready { .. } => RenderBufferSlotState::Ready,
            Self::Retained { .. } => RenderBufferSlotState::Retained,
        }
    }
}

#[derive(Debug)]
struct Slot<Scope> {
    id: RenderBufferId,
    generation: RenderLeaseGeneration,
    capacity_bytes: usize,
    state: SlotState<Scope>,
}

/// Host-owned bounded slot/lease state machine for out-of-band render payloads.
///
/// `Scope` is deliberately generic. The application composes this pool with its own product-owned
/// authority (for example `SessionAuthorityStamp`) without making this low-level resource crate
/// depend on session, engine, transport or UI types.
#[derive(Debug)]
pub struct RenderBufferPool<Scope> {
    limits: RenderBufferPoolLimits,
    slots: Vec<Slot<Scope>>,
    total_capacity_bytes: usize,
    outstanding_by_scope: BTreeMap<Scope, usize>,
}

impl<Scope> RenderBufferPool<Scope>
where
    Scope: Clone + Ord,
{
    #[must_use]
    pub fn new(limits: RenderBufferPoolLimits) -> Self {
        Self {
            limits,
            slots: Vec::new(),
            total_capacity_bytes: 0,
            outstanding_by_scope: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn limits(&self) -> RenderBufferPoolLimits {
        self.limits
    }

    pub fn acquire(
        &mut self,
        scope: Scope,
        requested_bytes: usize,
    ) -> Result<RenderWriteLease<Scope>, RenderBufferAcquireError> {
        if requested_bytes == 0 {
            return Err(RenderBufferAcquireError::ZeroByteRequest);
        }
        if requested_bytes > self.limits.max_slot_bytes {
            return Err(RenderBufferAcquireError::RequestExceedsSlotLimit {
                requested: requested_bytes,
                limit: self.limits.max_slot_bytes,
            });
        }

        let outstanding = self.outstanding_by_scope.get(&scope).copied().unwrap_or(0);
        if outstanding >= self.limits.max_outstanding_leases_per_scope {
            return Err(RenderBufferAcquireError::ScopeLeaseLimitReached {
                limit: self.limits.max_outstanding_leases_per_scope,
            });
        }

        let slot_index = self
            .best_reusable_slot(requested_bytes)
            .or_else(|| self.grow_reusable_slot(requested_bytes))
            .or_else(|| self.allocate_slot(requested_bytes))
            .ok_or(RenderBufferAcquireError::PoolExhausted)?;

        let slot = &mut self.slots[slot_index];
        let next_generation = slot
            .generation
            .0
            .checked_add(1)
            .map(RenderLeaseGeneration)
            .ok_or(RenderBufferAcquireError::LeaseGenerationExhausted { buffer_id: slot.id })?;
        slot.generation = next_generation;
        slot.state = SlotState::Leased {
            scope: scope.clone(),
            requested_bytes,
        };
        *self.outstanding_by_scope.entry(scope.clone()).or_insert(0) += 1;

        Ok(RenderWriteLease {
            id: RenderLeaseId {
                buffer_id: slot.id,
                generation: slot.generation,
            },
            scope,
            capacity_bytes: slot.capacity_bytes,
            requested_bytes,
        })
    }

    pub fn publish(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
        written_bytes: usize,
    ) -> Result<ReadyRenderBuffer<Scope>, RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let (lease_scope, requested_bytes) = match &self.slots[slot_index].state {
            SlotState::Leased {
                scope,
                requested_bytes,
            } => (scope, *requested_bytes),
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Leased,
                    actual: state.public_state(),
                });
            }
        };
        if lease_scope != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        if written_bytes == 0 {
            return Err(RenderBufferTransitionError::ZeroByteCompletion);
        }
        if written_bytes > requested_bytes {
            return Err(RenderBufferTransitionError::CompletionExceedsRequest {
                written: written_bytes,
                requested: requested_bytes,
            });
        }

        let published_scope = lease_scope.clone();
        self.decrement_outstanding(&published_scope);
        let slot = &mut self.slots[slot_index];
        slot.state = SlotState::Ready {
            scope: published_scope.clone(),
            written_bytes,
        };
        Ok(ReadyRenderBuffer {
            id: lease_id,
            scope: published_scope,
            capacity_bytes: slot.capacity_bytes,
            written_bytes,
        })
    }

    pub fn retain(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let (ready_scope, written_bytes) = match &self.slots[slot_index].state {
            SlotState::Ready {
                scope,
                written_bytes,
            } => (scope, *written_bytes),
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Ready,
                    actual: state.public_state(),
                });
            }
        };
        if ready_scope != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        self.slots[slot_index].state = SlotState::Retained {
            scope: ready_scope.clone(),
            written_bytes,
        };
        Ok(())
    }

    pub fn recycle(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let state = &self.slots[slot_index].state;
        let owner = match state {
            SlotState::Ready { scope, .. } | SlotState::Retained { scope, .. } => scope,
            _ => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Ready,
                    actual: state.public_state(),
                });
            }
        };
        if owner != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        self.slots[slot_index].state = SlotState::Available;
        Ok(())
    }

    pub fn cancel(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let owner = match &self.slots[slot_index].state {
            SlotState::Leased { scope, .. } => scope,
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Leased,
                    actual: state.public_state(),
                });
            }
        };
        if owner != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        let owner = owner.clone();
        self.decrement_outstanding(&owner);
        self.slots[slot_index].state = SlotState::Available;
        Ok(())
    }

    /// Invalidate every slot associated with a dead/replaced caller scope and make its capacity
    /// reusable. Any later completion carrying an old generation is rejected once the slot is
    /// leased again; before reuse it is rejected because the slot is no longer leased.
    pub fn invalidate_scope(&mut self, scope: &Scope) -> RenderScopeInvalidation {
        let mut result = RenderScopeInvalidation::default();
        for slot in &mut self.slots {
            let reclaimed = match &slot.state {
                SlotState::Leased { scope: owner, .. } if owner == scope => {
                    result.leased_reclaimed += 1;
                    true
                }
                SlotState::Ready { scope: owner, .. } if owner == scope => {
                    result.ready_reclaimed += 1;
                    true
                }
                SlotState::Retained { scope: owner, .. } if owner == scope => {
                    result.retained_reclaimed += 1;
                    true
                }
                _ => false,
            };
            if reclaimed {
                slot.state = SlotState::Available;
            }
        }
        self.outstanding_by_scope.remove(scope);
        result
    }

    #[must_use]
    pub fn slot_state(&self, buffer_id: RenderBufferId) -> Option<RenderBufferSlotState> {
        self.slot(buffer_id).map(|slot| slot.state.public_state())
    }

    #[must_use]
    pub fn slot_capacity_bytes(&self, buffer_id: RenderBufferId) -> Option<usize> {
        self.slot(buffer_id).map(|slot| slot.capacity_bytes)
    }

    #[must_use]
    pub fn stats(&self) -> RenderBufferPoolStats {
        let mut stats = RenderBufferPoolStats {
            total_capacity_bytes: self.total_capacity_bytes,
            slot_count: self.slots.len(),
            ..RenderBufferPoolStats::default()
        };
        for slot in &self.slots {
            match slot.state {
                SlotState::Available => stats.available_slots += 1,
                SlotState::Leased { .. } => stats.leased_slots += 1,
                SlotState::Ready { .. } => stats.ready_slots += 1,
                SlotState::Retained { .. } => stats.retained_slots += 1,
            }
        }
        stats
    }

    fn best_reusable_slot(&self, requested_bytes: usize) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| {
                matches!(slot.state, SlotState::Available)
                    && slot.capacity_bytes >= requested_bytes
            })
            .min_by_key(|(_, slot)| slot.capacity_bytes)
            .map(|(index, _)| index)
    }

    fn grow_reusable_slot(&mut self, requested_bytes: usize) -> Option<usize> {
        let (index, current_capacity) = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| matches!(slot.state, SlotState::Available))
            .filter(|(_, slot)| slot.capacity_bytes < requested_bytes)
            .max_by_key(|(_, slot)| slot.capacity_bytes)
            .map(|(index, slot)| (index, slot.capacity_bytes))?;
        let additional = requested_bytes.checked_sub(current_capacity)?;
        let new_total = self.total_capacity_bytes.checked_add(additional)?;
        if new_total > self.limits.max_total_bytes {
            return None;
        }
        self.slots[index].capacity_bytes = requested_bytes;
        self.total_capacity_bytes = new_total;
        Some(index)
    }

    fn allocate_slot(&mut self, requested_bytes: usize) -> Option<usize> {
        if self.slots.len() >= self.limits.max_slots {
            return None;
        }
        let new_total = self.total_capacity_bytes.checked_add(requested_bytes)?;
        if new_total > self.limits.max_total_bytes {
            return None;
        }
        let id = RenderBufferId(u32::try_from(self.slots.len()).ok()?);
        self.slots.push(Slot {
            id,
            generation: RenderLeaseGeneration(0),
            capacity_bytes: requested_bytes,
            state: SlotState::Available,
        });
        self.total_capacity_bytes = new_total;
        Some(self.slots.len() - 1)
    }

    fn validate_generation(
        &self,
        lease_id: RenderLeaseId,
    ) -> Result<usize, RenderBufferTransitionError> {
        let index = usize::try_from(lease_id.buffer_id.0)
            .map_err(|_| RenderBufferTransitionError::UnknownBuffer(lease_id.buffer_id))?;
        let Some(slot) = self.slots.get(index) else {
            return Err(RenderBufferTransitionError::UnknownBuffer(lease_id.buffer_id));
        };
        if slot.id != lease_id.buffer_id {
            return Err(RenderBufferTransitionError::UnknownBuffer(lease_id.buffer_id));
        }
        if slot.generation != lease_id.generation {
            return Err(RenderBufferTransitionError::StaleLease {
                buffer_id: lease_id.buffer_id,
                expected_generation: slot.generation,
                actual_generation: lease_id.generation,
            });
        }
        Ok(index)
    }

    fn slot(&self, buffer_id: RenderBufferId) -> Option<&Slot<Scope>> {
        let index = usize::try_from(buffer_id.0).ok()?;
        let slot = self.slots.get(index)?;
        (slot.id == buffer_id).then_some(slot)
    }

    fn decrement_outstanding(&mut self, scope: &Scope) {
        let mut remove = false;
        if let Some(outstanding) = self.outstanding_by_scope.get_mut(scope) {
            *outstanding = outstanding.saturating_sub(1);
            remove = *outstanding == 0;
        }
        if remove {
            self.outstanding_by_scope.remove(scope);
        }
    }
}
