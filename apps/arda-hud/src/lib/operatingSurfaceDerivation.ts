import { type JsonRecord, type ArdaBundle } from './ardaSource'
import { asRecord, asArray, getString, getNumber, getBoolean, formatMetric } from "./ardaSurfaces"
import { type ReviewGateItem } from "../components/arda/types"
import {ArdaFleetHealth} from "../scene/workstations/adapters/ardaAdapter"
import {OperatingSurfaceLaneReport} from './ardaTypes'
import {primarySigilForSource} from "./soterionRender"

export function getKnowledgeMap(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  entries: Array<{
    path: string
    title: string
    classification: string
    canonicalHome: string
    authority: string
    domain: string
    glyph: string
  }>
  digest: Array<{
    sourceId: string
    title: string
    status: string
    sourceType: string
    tags: string[]
    summary: string
  }>
  deepGraph: Array<{
    sourceId: string
    confidence: string
    triadPassed: boolean
    nodeCount: number
    edgeCount: number
    labels: string[]
  }>
  policyReadiness: Array<{
    sourceId: string
    readiness: string
    confidence: string
    blockers: string[]
    triadPassed: boolean
  }>
  policySummary: {
    status: string
    policyReadyTotal: number
    referenceOnlyTotal: number
    reviewPressureTotal: number
    nextOperatorAction: string
    promotionPreviewAvailable: boolean
    governanceGate: string
  }
  missingProjections: Array<{
    sectionId: string
    sectionTitle: string
    owner: string
    missing: string[]
  }>
} {
  const records = bundle.knowledgeTriage
  const counts = records.reduce<Record<string, number>>((acc, record) => {
    const classification = getString(record.classification, 'unknown')
    acc[classification] = (acc[classification] ?? 0) + 1
    return acc
  }, {})
  const priority = ['active', 'memory_seed', 'product_seed', 'delete_candidate', 'superseded', 'archive', 'reference']
  const summary = [
    { label: 'Total', value: `${records.length}` },
    ...priority
      .filter((classification) => counts[classification] !== undefined)
      .map((classification) => ({ label: classification, value: `${counts[classification]}` })),
    { label: 'Digest', value: `${bundle.athenaDigest.length}` },
    { label: 'Deep Graph', value: `${bundle.athenaDeepGraph.length}` },
    { label: 'Policy Readiness', value: `${bundle.athenaPolicyReadiness.length}` },
  ]
  const athenaPolicySummary = asRecord(bundle.athenaRuntime?.policy_readiness_summary)
  const policySummary = {
    status: getString(athenaPolicySummary?.status, 'unknown'),
    policyReadyTotal: getNumber(athenaPolicySummary?.policy_ready_total, 0),
    referenceOnlyTotal: getNumber(athenaPolicySummary?.reference_only_total, 0),
    reviewPressureTotal: getNumber(athenaPolicySummary?.review_pressure_total, 0),
    nextOperatorAction: getString(athenaPolicySummary?.next_operator_action, 'refresh_athena_digest'),
    promotionPreviewAvailable: getBoolean(athenaPolicySummary?.promotion_preview_available, false),
    governanceGate: getString(athenaPolicySummary?.governance_gate, 'human_review_required'),
  }
  const entries = records
    .filter((record) => getString(record.classification, '') !== 'delete_candidate')
    .slice(0, 10)
    .map((record) => {
      const soterion = asRecord(record.soterion)
      return {
        path: getString(record.path, 'unknown'),
        title: getString(record.title, getString(record.path, 'Untitled')),
        classification: getString(record.classification, 'unknown'),
        canonicalHome: getString(record.canonical_home, 'unknown'),
        authority: getString(record.authority, 'unknown'),
        domain: getString(record.domain, 'unknown'),
        glyph: getString(soterion?.glyph, primarySigilForSource('athena')),
      }
    })
  const digest = bundle.athenaDigest.slice(-10).reverse().map((record) => {
    const shallow = asRecord(record.shallow)
    const tags = asArray(shallow?.relevance_tags)
      .map((tag) => getString(tag, ''))
      .filter((tag) => tag.length > 0)
    return {
      sourceId: getString(record.source_id, getString(record.id, 'unknown')),
      title: getString(shallow?.title, getString(record.url, getString(record.event, 'ATHENA digest event'))),
      status: getString(record.digest_status, getString(record.status, getString(record.event, 'unknown'))),
      sourceType: getString(record.source_type, getString(record.agent, 'unknown')),
      tags,
      summary: getString(shallow?.summary, getString(record.reason, 'No digest summary available.')),
    }
  })
  const deepGraph = bundle.athenaDeepGraph.slice(-10).reverse().map((record) => {
    const nodes = asArray(record.nodes)
    const edges = asArray(record.edges)
    const labels = nodes
      .map((node) => asRecord(node))
      .filter((node): node is JsonRecord => node !== null)
      .map((node) => getString(node.label, getString(node.id, '')))
      .filter((label) => label.length > 0)
      .slice(0, 4)
    return {
      sourceId: getString(record.source_id, 'unknown'),
      confidence: formatMetric(getNumber(record.confidence, 0)),
      triadPassed: getBoolean(record.triad_passed, false),
      nodeCount: nodes.length,
      edgeCount: edges.length,
      labels,
    }
  })
  const policyReadiness = bundle.athenaPolicyReadiness.slice(-10).reverse().map((record) => {
    const gate = asRecord(record.gate)
    const observed = asRecord(gate?.observed)
    const blockers = asArray(gate?.blockers)
      .map((blocker) => getString(blocker, ''))
      .filter((blocker) => blocker.length > 0)
    return {
      sourceId: getString(record.source_id, getString(gate?.source_id, 'unknown')),
      readiness: getString(record.policy_readiness, 'unknown'),
      confidence: formatMetric(getNumber(observed?.confidence, 0)),
      blockers,
      triadPassed: getBoolean(observed?.triad_passed, false),
    }
  })
  const missingProjections = bundle.sections
    .filter((section) => (section.missing_projections ?? []).length > 0)
    .map((section) => ({
      sectionId: section.id,
      sectionTitle: section.title,
      owner: section.owner,
      missing: section.missing_projections ?? [],
    }))
  return { summary, entries, digest, deepGraph, policyReadiness, policySummary, missingProjections }
}

export function getOperatingSurfaceReports(
  bundle: ArdaBundle,
  reviewGateItems: ReviewGateItem[],
  fleetHealth: ArdaFleetHealth,
  knowledgeMap: ReturnType<typeof getKnowledgeMap>,
): OperatingSurfaceLaneReport[] {
  const hadesNightly = asRecord(bundle.hadesNightlyOperations)
  const hadesArtifacts = asRecord(hadesNightly?.artifacts)
  const rumilCommands = asRecord(hadesNightly?.commands)
  const rumilOrganization = asRecord(rumilCommands?.rumil_organization_maintenance)
  const athenaCounts = asRecord(asRecord(bundle.athenaRuntime?.knowledge)?.counts)
  const businessCounts = asRecord(asRecord(bundle.businessRuntime)?.counts)
  const configProfiles = asArray(bundle.configWalkthroughProfiles?.profiles)
  const sourceMapSections = bundle.sections
  const missingProjectionCount = sourceMapSections.reduce((total, section) => total + (section.missing_projections?.length ?? 0), 0)
  const pendingReviewCount = reviewGateItems.filter((item) => item.status.includes('pending') || item.status.includes('review')).length
  const executionAllowed = bundle.automationStatus?.execution_allowed === true
  const hadesStatus = getString(hadesNightly?.status, 'missing')
  const athenaReferenceOnly = knowledgeMap.policySummary.referenceOnlyTotal || getNumber(athenaCounts?.reference_only_recent, bundle.athenaDigest.length)
  const athenaPolicyReady = knowledgeMap.policySummary.policyReadyTotal || getNumber(athenaCounts?.policy_ready_recent, 0)
  const candidatePreviewTotal = getString(rumilOrganization?.stdout_tail, '').match(/candidate_preview_total=(\d+)/)?.[1] ?? 'unknown'
  const brokenLocalLinks = getString(rumilOrganization?.stdout_tail, '').match(/broken_local_links=(\d+)/)?.[1] ?? 'unknown'

  return [
    {
      lane: 'Now',
      status: executionAllowed && pendingReviewCount === 0 ? 'partial' : 'gap',
      current: `Automation posture is ${bundle.automationStatus?.posture ?? 'unknown'} and ${pendingReviewCount} review items need attention.`,
      gap: 'The first screen is still assembled from modules instead of a single operator answer for mode, attention, active work, and safe action.',
      next: 'Promote this review plus executive, decisions, and health summaries into a dedicated Now command surface.',
      evidence: ['automationStatus.ts', 'ExecutiveOverviewModule.tsx', 'ReviewGateWorkstation.tsx'],
    },
    {
      lane: 'Work',
      status: hadesStatus === 'pass' ? 'partial' : 'gap',
      current: `HADES latest nightly is ${hadesStatus}; organization plan preview reports ${candidatePreviewTotal} candidates.`,
      gap: 'Planning, recurring jobs, scheduled operations, and manual run controls are split across Systems, Planning, and Operations panels.',
      next: 'Create a Work lane that lists daily/project queues, scheduled jobs, blocked jobs, receipts, and safe run controls.',
      evidence: ['core/state/hades_nightly_operations.json', 'Planning module', getString(hadesArtifacts?.organization_plan, 'organization plan missing')],
    },
    {
      lane: 'Decisions',
      status: reviewGateItems.length > 0 ? 'partial' : 'gap',
      current: `${reviewGateItems.length} review packets are projected from Arandur, HADES, and ATHENA sources.`,
      gap: 'Decision packets exist, but they are nested under Governance Controls and not exposed as a top-level human augmentation lane.',
      next: 'Make Decisions a top-level view with policy reason, consequence, delegation, and evidence per packet.',
      evidence: ['ArandurApprovalWorkstation.tsx', 'ReviewGateWorkstation.tsx', 'data/hades/lifecycle_review_queue.jsonl'],
    },
    {
      lane: 'Knowledge',
      status: bundle.athenaRuntime ? (knowledgeMap.policySummary.reviewPressureTotal > 0 ? 'partial' : 'ready') : 'gap',
      current: `ATHENA shows ${athenaPolicyReady} policy-ready items, ${athenaReferenceOnly} reference-only items, and policy status ${knowledgeMap.policySummary.status}.`,
      gap: 'Knowledge data is present but largely lives inside Systems; MNEMOSYNE continuity and human ingestion are not unified here yet.',
      next: `Next operator action: ${knowledgeMap.policySummary.nextOperatorAction}. Split Knowledge into research status, memory continuity, source freshness, conflicts, citations, and review queues.`,
      evidence: ['core/state/athena_runtime.json', 'data/athena/digest.jsonl', 'KnowledgeMapPanel.tsx'],
    },
    {
      lane: 'Health',
      status: fleetHealth.unexpectedOffline === 0 ? 'partial' : 'gap',
      current: `${fleetHealth.liveTargets}/${fleetHealth.totalTargets} fleet targets live, ${fleetHealth.routableProviders} routable providers, ${fleetHealth.unexpectedOffline} unexpected offline.`,
      gap: 'Provider mesh, Manwe routing, Hermes, systemd, fleet, and drift are displayed as system panels rather than one health posture.',
      next: 'Make Health summarize runtime/service/provider state first, with drilldowns for routing ownership and drift.',
      evidence: ['SystemsModule.tsx', 'core/state/operator_runtime_status.json', 'core/state/manwe_router.json'],
    },
    {
      lane: 'Business',
      status: bundle.businessRuntime ? 'partial' : 'gap',
      current: `Business runtime has ${getNumber(businessCounts?.client_records_total, 0)} client records and ${getNumber(businessCounts?.state_keys_total, 0)} state keys.`,
      gap: 'Business exists as a reserved/runtime module but does not yet connect active projects, economics, commitments, and readiness.',
      next: 'Expand Business around active projects, JouleWork accounting, risks, opportunities, and customer-facing readiness.',
      evidence: ['BusinessModule.tsx', 'core/state/business_runtime.json', 'src/lib/serviceLibraryBooks.ts'],
    },
    {
      lane: 'Evidence',
      status: missingProjectionCount === 0 ? 'partial' : 'gap',
      current: `${sourceMapSections.length} source-map sections loaded, ${missingProjectionCount} missing projections, ${brokenLocalLinks} broken local links in latest HADES check.`,
      gap: 'Provenance badges exist, but audit reports, receipts, source map, freshness, and known gaps are not one trust surface.',
      next: 'Create an Evidence lane with source map, receipts, audits, freshness, and validation status as first-class objects.',
      evidence: ['core/state/system_source_map.json', getString(hadesArtifacts?.markdown_link_check, 'markdown link check missing'), 'DataSourceDetailsPanel.tsx'],
    },
    {
      lane: 'Settings',
      status: configProfiles.length > 0 ? 'partial' : 'gap',
      current: `${configProfiles.length} guided config profiles are available; setup readiness state is ${bundle.setupConsoleReadiness ? 'loaded' : 'missing'}.`,
      gap: 'Settings contains configuration and monitor controls, but onboarding is not yet a guided first-run operating path.',
      next: 'Promote setup readiness, provider/API setup, governance defaults, service checks, repair actions, and exportable status.',
      evidence: ['SettingsModule.tsx', 'src/lib/configWalkthrough.ts', 'core/state/setup_console_readiness.json'],
    },
  ]
}