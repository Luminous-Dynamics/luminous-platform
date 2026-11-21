#!/usr/bin/env python3
"""Mutation tests for MCP descriptor and per-invocation authority integrity."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import check_mcp_tool_contract_integrity as checker  # noqa: E402

def inputs():
 s=checker.load(checker.BASE/"mcp-tool-contract-integrity.fixtures.json")
 schema=checker.load(checker.BASE/"mcp-tool-contract-integrity.schema.json")
 return s,schema

def case(suite,sid):
 return copy.deepcopy(next(c for c in suite["cases"] if c["scenario_id"]==sid))

def reject(suite,schema):
 try:checker.validate_suite(suite,schema)
 except ValueError:return
 raise AssertionError("mutated MTCI suite was accepted")

class MTCITests(unittest.TestCase):
 def test_frozen_descriptor_and_authority_panel(self):
  s,schema=inputs()
  self.assertEqual(checker.validate_suite(s,schema),{"cases":14,"positive_control":1,"negative_control":12,"indeterminate_control":1})

 def test_input_schema_rug_pull_is_detected(self):
  s,_=inputs();c=case(s,"MTCI-001")
  self.assertFalse(checker.descriptor_ok(c))

 def test_annotation_change_is_detected_even_when_description_is_same(self):
  s,_=inputs();c=case(s,"MTCI-003")
  self.assertEqual(c["reviewed_descriptor"]["description_digest"],c["presented_descriptor"]["description_digest"])
  self.assertFalse(checker.descriptor_ok(c))

 def test_server_origin_substitution_is_detected(self):
  s,_=inputs();self.assertFalse(checker.descriptor_ok(case(s,"MTCI-004")))

 def test_same_name_different_server_tool_id_is_detected(self):
  s,_=inputs();c=case(s,"MTCI-006")
  self.assertEqual(c["reviewed_descriptor"]["tool_name"],c["presented_descriptor"]["tool_name"])
  self.assertFalse(checker.descriptor_ok(c))

 def test_wrong_token_audience_is_denied(self):
  s,_=inputs();self.assertFalse(checker.authority_ok(case(s,"MTCI-007")))

 def test_wrong_issuer_is_denied(self):
  s,_=inputs();self.assertFalse(checker.authority_ok(case(s,"MTCI-008")))

 def test_wildcard_scope_is_denied(self):
  s,_=inputs();self.assertFalse(checker.authority_ok(case(s,"MTCI-009")))

 def test_consumed_nonce_is_denied(self):
  s,_=inputs();self.assertFalse(checker.authority_ok(case(s,"MTCI-010")))

 def test_session_handle_is_not_authentication(self):
  s,_=inputs();self.assertFalse(checker.authority_ok(case(s,"MTCI-011")))

 def test_approval_does_not_transfer_to_replaced_descriptor(self):
  s,_=inputs();c=case(s,"MTCI-012")
  self.assertNotEqual(c["approval"]["reviewed_descriptor_digest"],c["presented_descriptor"]["canonical_descriptor_digest"])
  self.assertFalse(checker.descriptor_ok(c))

 def test_unknown_server_identity_remains_indeterminate(self):
  s,_=inputs();c=case(s,"MTCI-013")
  self.assertEqual(checker.evaluate({**c,"_effects":[{"effect_id":e["effect_id"]} for e in s["effects"]]})["decision"],"hold")

 def test_positive_control_is_accepted(self):
  s,_=inputs();c=case(s,"MTCI-014")
  self.assertTrue(checker.descriptor_ok(c));self.assertTrue(checker.authority_ok(c))

 def test_expected_audience_cannot_be_redefined_by_presented_authority(self):
  s,schema=inputs();c=case(s,"MTCI-014")
  c["authority"]["expected_audience"]="audience:attacker"
  c["authority"]["token_audience"]="audience:attacker"
  reject(s,schema)

 def test_required_scope_set_is_pinned_by_approval(self):
  s,schema=inputs();c=case(s,"MTCI-014")
  c["authority"]["required_scopes"]=["records.write"]
  c["authority"]["granted_scopes"]=["records.write"]
  reject(s,schema)

 def test_denied_invocation_cannot_be_observed_as_successful_effect(self):
  s,schema=inputs();c=case(s,"MTCI-001")
  c["oracle"]["observation"]="observed";c["oracle"]["observed_effect_ids"]=["effect:records-read"];c["oracle"]["complete"]=True;c["oracle"]["independent"]=True
  reject(s,schema)

 def test_invocation_must_bind_approved_descriptor_digest(self):
  s,schema=inputs();c=case(s,"MTCI-014")
  c["invocation"]["descriptor_digest"]= "sha256:"+"9"*64
  reject(s,schema)

 def test_missing_required_scope_cannot_be_recovered_by_other_authority_fields(self):
  s,schema=inputs();c=case(s,"MTCI-007")
  c["authority"]["token_audience"]=c["authority"]["expected_audience"]
  c["authority"]["required_scopes"]=["records.write"]
  reject(s,schema)

if __name__=="__main__":unittest.main()
