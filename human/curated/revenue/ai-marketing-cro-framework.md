---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "athena_curated_intake"
  owner: "ATHENA"
  status: "active"
  last_reviewed: "2026-05-26"
athena:
  source_path: "human/Untitled 1.md"
  intake_date: "2026-05-26"
  category: "revenue_reference"
  promotion_status: "curated_reference"
  tags: ["marketing", "cro", "growth", "ai-services"]
  phase7_boundary: "phase_7_not_reopened"
  rationale: "Revenue reference for marketing/CRO system design; promote to active offer only after proof and scope selection."
---

> 🜏 Soterion: 📜 athena_curated_intake | owner: ATHENA | status: active | reviewed: 2026-05-26

# AI marketing and CRO framework

## ATHENA classification

- Source path: `human/Untitled 1.md`
- Category: `revenue_reference`
- Promotion status: `curated_reference`
- Rationale: Revenue reference for marketing/CRO system design; promote to active offer only after proof and scope selection.
- Phase boundary: Phase 7 remains closed; this is human-vault intake / Phase 8+ autonomy planning material where applicable.

## Original imported content

139 marketing tactics and a full CRO framework that replace a $10,000/month agency. One repo $0

Marketing agency charges $10,000 a month. Senior marketer - $15,000. Consultant - $300 an hour.

Someone collected everything they know into one free repo and packaged it for AI agents. 139 growth tactics, 12 SEO playbooks, frameworks for copywriting, CRO, pricing and A/B testing.

> Github link at the end of the article

An agent with this system does in a day what a marketer spends a week on. Here's what's inside.

The insight that makes everything else work

Every skill in this repo starts with one file: product-marketing-context.md. Before the agent writes a single word of copy or analyzes a single landing page it reads this file first.

The file contains everything an experienced marketer would spend their first week learning: product overview, target audience, personas, pain points, competitive landscape, differentiation, objections, switching dynamics, customer language, brand voice, proof points and goals.

The key word is customer language. Verbatim words from real customers - pulled from interviews, reviews and support tickets - are worth more than any polished internal description because they reflect how customers actually think and talk.

The workflow:

```text
Claude reads your codebase, landing pages and docs
→ auto-drafts the product marketing context
→ you review and correct what's wrong
→ every other skill reads this context before acting
```

Product context prompt - run this before any other marketing task:

```text
Study the repo: README, landing pages, marketing copy,
package.json, docs.

Draft:
- Product overview and core value proposition
- Target audience and ICP
- Personas with jobs to be done
- Pain points and switching forces (push/pull/habit/anxiety)
- Competitive landscape and differentiation
- Common objections and how to handle them
- Customer language — verbatim words from reviews and support
- Proof points and social proof available
- Current goals and success metrics

Then ask: what needs correcting or what is missing?
```

Without this file the agent writes generic output. With it the agent knows your ICP, your objections, your proof points and your customer's exact words.

Switching Forces: why customers switch and why they don't

The most underrated framework in the entire repo is Switching Forces - four forces that explain why customers change products and why they don't.

```text
Push    → what's frustrating enough about the current solution
           to make someone start looking for alternatives

Pull    → what's attractive about the new solution
           that makes someone want to switch

Habit   → what keeps people stuck with what they have
           even when they're unhappy

Anxiety → what fears about switching hold people back
           even when they want to move
```

Most marketing focuses only on Pull - here's why our product is great. The agents in this repo are built to address all four. A landing page that only talks about features ignores the three forces that actually prevent conversion. An agent that knows your Switching Forces writes copy that removes anxiety, acknowledges habits and addresses the real push that brought someone to your page in the first place.

## 

Part 1 - Copywriting: clarity beats cleverness every time

The copywriting skill positions the agent as a conversion copywriter with four non-negotiable principles: clarity over cleverness, benefits over features, specificity over vagueness and customer language over company language. Not "save time" but "cut weekly reporting from 4 hours to 15 minutes."

[

](https://x.com/noisyb0y1/article/2050523761481285898/media/2049812762415513600)

Headline formulas that actually work:

```text
{Achieve outcome} without {pain point}
Turn {input} into {outcome}
Never {unpleasant event} again
The {category} for {audience}
{Number} {people} use {product} to {outcome}
```

CTA formula:

```text
[Action Verb] + [What They Get] + [Qualifier if needed]

"Get the Complete Checklist"
"See Pricing for My Team"
"Start My Free Trial"
```

Copywriting prompt:

```text
Write page copy for [page type].
Primary action: [conversion goal].
Audience: [ICP].
Problem: [pain point].
Objections: [hesitations].
Traffic source: [ads / organic / email].
Use customer language, be specific, avoid jargon.
Output: headline, subheadline, CTA, sections, annotations, alternatives.
```

## 

Part 2 - CRO: a page is a sequence of arguments not a collection of blocks

The CRO skill analyzes landing pages in this exact order of impact: value proposition clarity, headline effectiveness, CTA placement and copy, visual hierarchy, trust signals, objection handling, friction points.

The 7-question framework every page should pass:

```text
1. Is it clear in 5 seconds what this is and why it matters?
2. Does the headline communicate specific value?
3. Is there one primary CTA?
4. Can you scan the page visually?
5. Are there trust signals next to the CTA?
6. Are objections addressed?
7. Where is the friction — form, next steps, mobile, load time?
```

CRO audit prompt:

```text
Audit this page for conversion.

Identify: page type, primary conversion goal, traffic source.

Analyze in order:
1. Value proposition — clear in 5 seconds?
2. Headline — specific outcome or vague claim?
3. CTA — one primary action, right copy, right placement?
4. Visual hierarchy — scannable without reading?
5. Trust signals — present near CTA?
6. Objections — addressed before they block conversion?
7. Friction — form fields, next steps, mobile, load time?

Output: quick wins, high-impact changes, test ideas, copy alternatives.
```

[

](https://x.com/noisyb0y1/article/2050523761481285898/media/2049811684609478656)

## 

Part 3 - A/B testing: hypothesis first, button color last

Every test starts with a structured hypothesis:

```text
Because [observation/data],
we believe [change]
will cause [expected outcome]
for [audience].
We'll know this is true when [primary metric + guardrails].
```

ICE score for prioritizing what to test:

```text
ICE Score = (Impact + Confidence + Ease) / 3
```

Test duration formula:

```text
Duration (days) = (Sample per variant × variants) / (Daily traffic × % exposed)
```

Rule of thumb: more than 60 days - consider alternatives. Less than 14 days - usually feasible.

A/B test hypothesis prompt:

```text
Because [observation/data],
we believe [change]
will cause [expected outcome]
for [audience].
We'll know this is true when [primary metric changes by X%]
without [guardrail metric] degrading.

Calculate required sample size and test duration.
ICE score: Impact [1-10] + Confidence [1-10] + Ease [1-10] / 3.
```

## 

Part 4 - Pricing: three axes, one framework

Most founders set pricing by looking at competitors and picking a number in the middle. The pricing skill builds pricing on three axes: packaging - what features and limits go in each tier, pricing metric - what unit customers pay for and price point - the actual number.

The principle that matters most: price should be set based on value delivered not cost to serve. It should sit between two numbers - the next best alternative and the perceived value of your product.

Good-Better-Best structure:

```text
Good   → entry tier, core features, limited usage
Better → recommended tier, full features, reasonable limits
Best   → premium tier, advanced features, 2-3x Better price
```

[

](https://x.com/noisyb0y1/article/2050523761481285898/media/2049812474891763712)

Pricing psychology the repo covers: anchoring makes the real price feel smaller by showing a higher price first. Decoy effect makes customers choose the top tier by placing a clearly worse middle tier. Charm pricing - $49 feels meaningfully cheaper than $50. Round pricing - $100 signals confidence and premium positioning.

Pricing page elements that convert:

```text
Tier comparison table with recommended badge
Monthly/annual toggle with annual discount visible
FAQ addressing the most common objections
Guarantee or free trial to reduce anxiety
Trust signals near the CTA
```

## 

Part 5 - AI SEO: getting cited beats getting ranked

Traditional SEO gets you ranked. AI SEO gets you cited by Google AI Overviews, ChatGPT, Perplexity, Claude, Gemini and Copilot. AI systems can cite a well-structured page even if it's not in position 1 because they evaluate quality, structure and relevance of individual passages.

[

](https://x.com/noisyb0y1/article/2050523761481285898/media/2049812837736861696)

Three pillars:

```text
1. Structure  → make content extractable
2. Authority  → make content citable
3. Presence   → be where AI systems look
```

Content block patterns that get cited:

```text
Definition blocks    → "What is X?"
Step-by-step blocks  → "How to X?"
Comparison tables    → "X vs Y"
Pros/cons blocks     → evaluation queries
FAQ blocks           → natural language questions
Statistic blocks     → claims with cited sources
```

The most underrated idea in the repo: add /pricing.md or /pricing.txt to your site so AI agents can programmatically read your pricing. If pricing is hidden behind JavaScript or "contact sales" an AI agent may simply exclude your product from a comparison. In the near future your site is read not just by humans and search bots but by buying agents.

AI SEO audit prompt:

```text
For each priority query, check whether we are cited in
Google AI Overviews, ChatGPT, and Perplexity.

Analyze which competitors are cited instead of us and why.

Rewrite our content into extractable blocks:
- Definition block for "what is X" queries
- Step-by-step block for "how to X" queries
- Comparison table for "X vs Y" queries
- FAQ block for natural language questions
- Statistics block with cited sources

Add schema markup, last-updated date, author attribution.
Create /pricing.md and /llms.txt for machine-readable access.

Output: citation gaps, content rewrites, structural fixes, files to create.
```

## 

Part 6 - Programmatic SEO: 12 playbooks for scaling content

12 playbooks for creating pages at scale:

```text
Templates      → "[type] template"
Curation       → "best [category]"
Conversions    → "[X] to [Y]"
Comparisons    → "[X] vs [Y]"
Examples       → "[type] examples"
Locations      → "[service] in [location]"
Personas       → "[product] for [audience]"
Integrations   → "[product A] [product B] integration"
Glossary       → "what is [term]"
Translations   → localized content
Directory      → "[category] tools"
Profiles       → "[entity name]"
```

The rule that matters most: 100 strong pages beats 10,000 thin pages.

## 

Part 7 - The growth idea library: 139 tactics, zero fluff

The most interesting tactics aren't the obvious ones:

Glossary Marketing - every industry term becomes an SEO page that ranks, brings in exactly the right audience and costs nothing after it's written. Each term is a permanent asset.

Engineering as Marketing - build a free calculator, generator or tool that solves a small version of the problem your paid product solves. It attracts exactly the right people, generates links and feeds directly into your acquisition funnel.

Importers as Marketing - "import from [competitor]" as a feature reduces switching friction to near zero. Build the bridge and make switching trivially easy.

Proprietary Data Content - data that only exists inside your product becomes a linkable asset when published. Your product sees patterns no one else can see and that data turned into a report attracts links, press and exactly the right audience.

Public Changelogs - publishing what you're shipping signals momentum. Customers who see consistent progress stay subscribed. Prospects who see active development convert faster.

Reddit Keyword Research - Reddit discussions contain the exact language customers use when no one is watching. "How do I stop losing track of client emails" is a better headline than "Email management for professionals" because it's what customers actually type when frustrated.

Founder Welcome Email - a personal email from the founder in the first 24 hours after signup has the highest open rate of any email in the sequence. It creates a human touchpoint that no automated email can replicate.

## 

Part 8 - The tool registry: from advice to execution

The repo doesn't stop at giving advice. It connects every skill to real tools: GA4, Google Search Console, Mixpanel, Amplitude, PostHog, Semrush, Ahrefs, HubSpot, Salesforce, Stripe, Mailchimp, Google Ads, Meta Ads, Hotjar, Optimizely - each with API, MCP, CLI and SDK availability noted.

The full execution chain:

```text
Agent skill   → decides what needs to be measured
Tool registry → finds the right tool
CLI           → runs the report / checks conversions / sends event
```

What this repo actually is

This isn't a collection of AI prompts. It's a new format for marketing work: context plus skills plus frameworks plus tool integrations plus measurement - applied sequentially by an agent that understands your product before it touches a single word of copy.

```text
Product Marketing Context  → agent understands who you are
Switching Forces           → agent knows why customers do or don't switch
Skill selection            → copywriting / CRO / SEO / pricing / ads
Framework execution        → checklist, hypothesis, ICE score
Tool integration           → GA4, GSC, HubSpot, Stripe via CLI
Measurement                → primary metric + guardrails + playbook
```

What you get after reading this

Marketing has always been expensive because it required people who know what to do and how to measure it. A senior marketer costs $8,000-15,000 per month. An agency charges $5,000-20,000 per project. A consultant bills $200-500 per hour.

This repo packages what those people know into markdown files that an AI agent reads and executes.

```text
139 growth tactics instead of a marketing consultant
12 pSEO playbooks instead of an SEO agency
CRO framework instead of a $3,000 UX audit
A/B hypothesis with ICE score instead of a product analyst
Pricing framework instead of a pricing consultant
Tool registry with CLI instead of a data analyst
```

The difference between marketing that converts and marketing that just exists is context, system and measurement discipline. This repo gives you all three.
