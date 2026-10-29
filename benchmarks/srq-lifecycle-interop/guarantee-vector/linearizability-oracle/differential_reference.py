"""Independent exhaustive reference semantics for tiny SRQ histories.

This deliberately enumerates every candidate permutation and implements its own
sequential transition/state encoding. Only input decoding is shared with the
optimized checker.
"""
from __future__ import annotations

from itertools import permutations

from linearizability_oracle import Operation, parse_history

RefState = tuple[tuple[str, str, str, str, str, str, str], ...]


def _ref_state_map(state: RefState) -> dict[str, tuple[tuple[str, str, str], str, str, str]]:
    return {
        item[0]: ((item[1], item[2], item[3]), item[4], item[5], item[6])
        for item in state
    }


def _ref_freeze(mapping: dict[str, tuple[tuple[str, str, str], str, str, str]]) -> RefState:
    return tuple(
        (key, intent[0], intent[1], intent[2], mode, effect_id, receipt_id)
        for key, (intent, mode, effect_id, receipt_id) in sorted(mapping.items())
    )


def _reference_ids_available(state: RefState, key: str, effect_id: str, receipt_id: str) -> bool:
    if not effect_id or not receipt_id:
        return False
    for item in state:
        existing_key, mode, existing_effect, existing_receipt = item[0], item[4], item[5], item[6]
        if existing_key != key and mode == "applied":
            if existing_effect == effect_id or existing_receipt == receipt_id:
                return False
    return True


def _reference_step(state: RefState, op: Operation) -> tuple[RefState, ...]:
    """Return every sequential state permitted by the written abstract contract."""
    by_key = _ref_state_map(state)
    previous = by_key.get(op.key)

    if op.result == "unknown":
        if previous is None:
            reserved = dict(by_key)
            reserved[op.key] = (op.intent, "none", "", "")
            ambiguous = dict(by_key)
            ambiguous[op.key] = (op.intent, "unknown", "", "")
            return tuple(dict.fromkeys((state, _ref_freeze(reserved), _ref_freeze(ambiguous))))
        old_intent, old_mode, old_effect, old_receipt = previous
        if old_intent != op.intent:
            return (state,)
        if old_mode == "none":
            unresolved = dict(by_key)
            unresolved[op.key] = (old_intent, "unknown", "", "")
            return tuple(dict.fromkeys((state, _ref_freeze(unresolved))))
        return (state,)

    if op.result == "key_conflict":
        if previous is not None and previous[0] != op.intent:
            return (state,)
        return ()

    assert op.result == "committed"
    assert op.effect_id is not None and op.receipt_id is not None
    if previous is None:
        if not _reference_ids_available(state, op.key, op.effect_id, op.receipt_id):
            return ()
        updated = dict(by_key)
        updated[op.key] = (op.intent, "applied", op.effect_id, op.receipt_id)
        return (_ref_freeze(updated),)

    old_intent, old_mode, old_effect, old_receipt = previous
    if old_intent != op.intent:
        return ()
    if old_mode == "applied":
        return (state,) if (old_effect == op.effect_id and old_receipt == op.receipt_id) else ()
    if not _reference_ids_available(state, op.key, op.effect_id, op.receipt_id):
        return ()
    updated = dict(by_key)
    updated[op.key] = (old_intent, "applied", op.effect_id, op.receipt_id)
    return (_ref_freeze(updated),)


def _respects_real_time(order: tuple[int, ...], operations: list[Operation]) -> bool:
    position = {operation_index: rank for rank, operation_index in enumerate(order)}
    for left, operation_left in enumerate(operations):
        for right, operation_right in enumerate(operations):
            if left != right and operation_left.end_ns < operation_right.start_ns:
                if position[left] >= position[right]:
                    return False
    return True


def reference_has_linearization(history: dict) -> tuple[bool, tuple[str, ...] | None]:
    """Brute-force every permutation; returns existence and one witness if found."""
    _, operations = parse_history(history)
    for order in permutations(range(len(operations))):
        if not _respects_real_time(order, operations):
            continue
        possible_states: set[RefState] = {()}
        for operation_index in order:
            next_states: set[RefState] = set()
            for state in possible_states:
                next_states.update(_reference_step(state, operations[operation_index]))
            possible_states = next_states
            if not possible_states:
                break
        if possible_states:
            return True, tuple(operations[index].op_id for index in order)
    return False, None


def reference_accepts_witness(history: dict, witness: list[str] | tuple[str, ...]) -> bool:
    _, operations = parse_history(history)
    if len(witness) != len(operations) or set(witness) != {op.op_id for op in operations}:
        return False
    by_id = {op.op_id: index for index, op in enumerate(operations)}
    order = tuple(by_id[op_id] for op_id in witness)
    if not _respects_real_time(order, operations):
        return False
    states: set[RefState] = {()}
    for index in order:
        next_states: set[RefState] = set()
        for state in states:
            next_states.update(_reference_step(state, operations[index]))
        states = next_states
        if not states:
            return False
    return bool(states)
