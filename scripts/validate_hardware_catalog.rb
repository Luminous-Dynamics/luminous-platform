#!/usr/bin/env ruby
# Validate catalog syntax and basic evidence-schema invariants.
# This does not verify remote links, licenses, supply, hardware, or qualification.

require "yaml"
require "uri"

path = ARGV.fetch(0, "hardware/catalog.yaml")

begin
  catalog = YAML.safe_load_file(path, permitted_classes: [], aliases: false)
rescue StandardError => e
  warn "ERROR: cannot parse #{path}: #{e.class}: #{e.message}"
  exit 1
end

errors = []
def require_fields(errors, obj, fields, label)
  unless obj.is_a?(Hash)
    errors << "#{label} must be a mapping"
    return
  end
  missing = fields.reject { |field| obj.key?(field) }
  errors << "#{label} is missing: #{missing.join(", ")}" unless missing.empty?
end

unless catalog.is_a?(Hash)
  warn "ERROR: catalog root must be a mapping"
  exit 1
end

require_fields(
  errors,
  catalog,
  %w[catalog_version research_snapshot catalog_status tracking_issue status_definitions records gaps],
  "catalog root"
)

if catalog["catalog_version"] != 1
  errors << "catalog_version must be integer 1 for this validator"
end

unless catalog["catalog_status"] == "initial-research-seed-not-exhaustive"
  errors << "catalog_status must state that this is an initial, non-exhaustive research seed"
end

definitions = catalog["status_definitions"]
unless definitions.is_a?(Hash)
  errors << "status_definitions must be a mapping"
end

allowed_maturity = %w[
  discovered
  reference_candidate
  prototype
  integration_qualified
  procurement_ready
  customer_authorized
]

records = catalog["records"]
unless records.is_a?(Array) && !records.empty?
  errors << "records must be a non-empty sequence"
  records = []
end

seen_ids = {}
record_fields = %w[
  id domain project record_type maturity current_part_or_revision sources
  hardware_source_state software_source_state licensing_state availability_state
  allowed_initial_use not_yet_claimed required_evidence
]
string_fields = %w[
  id domain project record_type current_part_or_revision hardware_source_state
  software_source_state licensing_state availability_state allowed_initial_use
]

records.each_with_index do |record, index|
  label = "records[#{index}]"
  require_fields(errors, record, record_fields, label)
  next unless record.is_a?(Hash)

  id = record["id"]
  unless id.is_a?(String) && id.match?(/\AHW-\d{3}\z/)
    errors << "#{label}.id must match HW-NNN"
  end
  if id.is_a?(String)
    errors << "duplicate record id #{id}" if seen_ids.key?(id)
    seen_ids[id] = true
  end

  string_fields.each do |field|
    value = record[field]
    errors << "#{label}.#{field} must be a non-empty string" unless value.is_a?(String) && !value.strip.empty?
  end

  unless allowed_maturity.include?(record["maturity"])
    errors << "#{label}.maturity must be one of: #{allowed_maturity.join(", ")}"
  end

  sources = record["sources"]
  unless sources.is_a?(Array) && !sources.empty?
    errors << "#{label}.sources must be a non-empty sequence of HTTPS URLs"
  else
    sources.each_with_index do |source, source_index|
      begin
        uri = URI.parse(source.to_s)
        unless uri.is_a?(URI::HTTPS) && !uri.host.to_s.empty?
          errors << "#{label}.sources[#{source_index}] is not an absolute HTTPS URL"
        end
      rescue URI::InvalidURIError
        errors << "#{label}.sources[#{source_index}] is not a valid URL"
      end
    end
  end

  %w[not_yet_claimed required_evidence].each do |field|
    value = record[field]
    unless value.is_a?(Array) && !value.empty? && value.all? { |item| item.is_a?(String) && !item.strip.empty? }
      errors << "#{label}.#{field} must be a non-empty sequence of non-empty strings"
    end
  end
end

gaps = catalog["gaps"]
unless gaps.is_a?(Array) && !gaps.empty?
  errors << "gaps must be a non-empty sequence"
  gaps = []
end

seen_gap_ids = {}
gaps.each_with_index do |gap, index|
  label = "gaps[#{index}]"
  require_fields(errors, gap, %w[id domain status exit_criteria], label)
  next unless gap.is_a?(Hash)

  id = gap["id"]
  unless id.is_a?(String) && id.match?(/\AGAP-\d{3}\z/)
    errors << "#{label}.id must match GAP-NNN"
  end
  if id.is_a?(String)
    errors << "duplicate gap id #{id}" if seen_gap_ids.key?(id)
    seen_gap_ids[id] = true
  end

  %w[domain status exit_criteria].each do |field|
    value = gap[field]
    errors << "#{label}.#{field} must be a non-empty string" unless value.is_a?(String) && !value.strip.empty?
  end
end

begin
  issue_uri = URI.parse(catalog["tracking_issue"].to_s)
  unless issue_uri.is_a?(URI::HTTPS) && !issue_uri.host.to_s.empty?
    errors << "tracking_issue must be an absolute HTTPS URL"
  end
rescue URI::InvalidURIError
  errors << "tracking_issue must be a valid HTTPS URL"
end

unless errors.empty?
  warn "Hardware catalog validation failed with #{errors.length} error(s):"
  errors.each { |error| warn " - #{error}" }
  exit 1
end

puts "Hardware catalog syntax/schema checks passed: #{records.length} records, #{gaps.length} explicit gaps."
puts "Scope: structural validation only; no remote source, license, availability, security, or hardware qualification is implied."
