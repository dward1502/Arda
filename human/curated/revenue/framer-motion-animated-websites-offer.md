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
  source_path: "human/Untitled 2.md"
  intake_date: "2026-05-26"
  category: "revenue_opportunity"
  promotion_status: "promoted_opportunity_sheet"
  tags: ["websites", "framer-motion", "claude-code", "smb"]
  phase7_boundary: "phase_7_not_reopened"
  rationale: "Valid service-offer candidate: animated websites for businesses; requires quality proof, delivery template, and pricing validation."
---

> 🜏 Soterion: 📜 athena_curated_intake | owner: ATHENA | status: active | reviewed: 2026-05-26

# Framer Motion animated websites offer

## ATHENA classification

- Source path: `human/Untitled 2.md`
- Category: `revenue_opportunity`
- Promotion status: `promoted_opportunity_sheet`
- Rationale: Valid service-offer candidate: animated websites for businesses; requires quality proof, delivery template, and pricing validation.
- Phase boundary: Phase 7 remains closed; this is human-vault intake / Phase 8+ autonomy planning material where applicable.

## Original imported content

claude code + framer motion: how to build $10k animated websites for businesses in 30 mins.

this is the definitive step-by-step guide to building high-motion animated websites for businesses using claude code and free tools, no prior skill or coding knowledge required.

> hey, i'm m0h, I research how people are using AI tools to make real money. this one's a profitable business model, not just a tutorial. bookmark it.

before the how-to, the stats. in the US alone:

— 1 in 4 small businesses still doesn't have a website — of the ones that do, most run generic templates that look like every other site in their niche — 75% of users judge a company's credibility based on website design alone — first impressions form in 50 milliseconds — custom business websites still cost $10,000+ to build the traditional way

a

[$10k-quality](https://x.com/search?q=%2410k-quality&src=cashtag_click)

animated site isn't vanity. it's the difference between being trusted and being skipped.

and with claude code + framer motion, you can ship one in 30 minutes — for yourself, or for clients paying agency rates.

## 

the stack

three tools. one free, one free with limits, one paid.

— claude code writes the code. anthropic's agent runs in your terminal, reads your project, ships features. starts at claude pro ($20/mo).

— motion animates the code. the open source react library formerly known as framer motion. free. npm install motion.

— nano banana pro generates the visuals motion moves around. google's gemini 3 pro image model. paid, free tier on google ai studio.

the trick is understanding they don't compete, they stack. claude code is the brain, motion is the muscle, nano banana is the art department.

miss one, the site looks like every other AI-built landing page on the internet.

## 

step 1 — install claude code (5 min)

go to your browser, search vs code, download the build for your os. it's free. open it, click the extensions icon on the left rail, search for claude code, install. sign in with your anthropic subscription.

then create a folder on your desktop animated-site works. open it in vs code. that's your project root. and if the vs code setup feels heavy for you, skip it. claude code runs inside the claude desktop app too.

go to

[claude.ai/download](https://claude.ai/download)

, install the desktop app for your os, sign in. open it, and you'll see claude code as a built-in option, no extension install, no terminal setup.

create your project folder anywhere on your machine, then point claude code at it through the app.

same agent. same capabilities. fewer moving parts.

claude code can run all of this for you. you just paste the code and it scaffolds.

```text
mkdir animated-site && cd animated-site
npx create-next-app@latest . --typescript --tailwind --app
npm install motion
```

## 

step 2 — install the skills that do the heavy lifting (3 min)

this is the part most tutorials skip. claude code's "skills" are markdown files that load context-specific best practices into the agent. for animated landing pages you want two:

1. a front-end design skill — the official anthropic one is fine, but tweaking it for landing-page conventions (fold structure, type scale, spacing rhythm) is what separates good from agency-tier output.
    
2. a video-to-website skill — this is the one that does the magic. it tells claude how to read a short product video and translate the motion into a scroll-driven web experience.
    

both were modified and packaged by nate, completely free. grab them from his skool community

[here](https://www.skool.com/ai-automation-society/about?el=the-new-nano-banana-2-claude-code-10k-websites)

, free signup, no paywall.

once you've downloaded the files, drop them into .claude/skills/ in your project. you can have claude code create the folder for you with the command:

```text
mkdir -p .claude/skills
```

then drag the .md files in through vs code's file explorer.

once they're in place, tell claude code: "use the skills in .claude/skills/ for this project." it'll read them and apply them to everything it builds from here.

```text
your-project/
├── .claude/
│   └── skills/
│       ├── frontend-design.md
│       └── video-to-website.md
├── public/
└── app/
```

## 

step 3 — generate your hero asset with nano banana pro (5 min)

this is where the apple-tier feel comes from.

open nano banana pro (gemini app or ai studio). generate two images of the product you're designing:

— one in its closed/normal state, — one in an exploded or x-ray state showing the components.

for example: a mechanical keyboard that's the keyboard intact, then the same keyboard with keycaps floating up and switches exposed beneath. for a camera, the body, then the body with the lens disassembled and floating in layers.

```text
prompt 1: "studio shot of a low-profile mechanical keyboard in
matte black with warm amber keycaps, floating, soft side lighting,
charcoal background, 4k product render"

prompt 2: "same keyboard but exploded view — keycaps floating up
in formation, switches exposed beneath, pcb and aluminum plate
separated and suspended, clean technical illustration style,
charcoal background"
```

now turn those two stills into a short reveal video, nano banana pro can interpolate, or you drop them into runway/kling for a 3-second transition. that 3-second clip is what claude code is going to read.

## 

step 4 — feed the video to claude code (2 min)

drag the keyboard reveal video into your project's /public/ folder. rename it something clean like reveal.mp4. then in the claude code chat:

```text
build me a product landing page using the video at /public/reveal.mp4.
the video shows a mechanical keyboard opening into an exploded view —
keycaps floating up, switches exposed, pcb and plate separated.
use motion for scroll-driven reveals,
follow the video-to-website skill,
match apple's product page layout structure.
```

that's the entire prompt. claude code reads the video, understands the motion arc, and builds a scroll-linked landing page where the video frames advance as the user scrolls.

dynamic copy fades in at each beat — "engineered for typists who care," "every switch, hand-tuned," "the keyboard you'll keep for a decade." that kind of thing.

you didn't write copy. you didn't pick a color scheme. claude inferred all of it from the asset.

## 

step 5 — the polish that signals "expensive" (5 min)

claude gets you 90% there. the last 10% is what makes a site feel premium instead of "AI-built."

you don't have to write any of this yourself. paste this into claude code:

> "go through the landing page and upgrade every animation. use a custom cubic-bezier easing curve [0.22, 1, 0.36, 1] instead of the default ease. for the hero, fade in from 40px below over 0.8s. for feature cards lower on the page, stagger the reveals by 0.1s as they scroll into view. make it feel like apple."

claude will edit the files. accept the diff. reload the page. it now looks like apple paid for it.

under the hood — what claude actually changed:

three small things separate generic from agency-tier. if you want to see what your prompt did:

1. the easing curve. default easing feels robotic — constant speed, no character. a custom cubic-bezier makes things glide. claude will have updated your hero's transition to look like this:

```text
tsx
transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
```

those four numbers tell motion to start fast and glide to a stop. single biggest tell between "vibe-coded" and "studio-built."

2. the entrance. elements rise into place instead of just fading in from nowhere:

```text
tsx
initial={{ opacity: 0, y: 40 }}
animate={{ opacity: 1, y: 0 }}
y: 40 means "start 40 pixels below where you'll end up." rises as it fades. feels deliberate.
```

3. the stagger. feature cards reveal one after another as the user scrolls — not all at once:

```text
tsx
<motion.div
  initial={{ opacity: 0, y: 60 }}
  whileInView={{ opacity: 1, y: 0 }}
  viewport={{ once: true, margin: "-100px" }}
  transition={{ duration: 0.6, delay: i * 0.1 }}
>
```

whileInView triggers when the element scrolls into view. delay: i * 0.1 means each card waits 0.1s longer than the previous. that's the staggered reveal that costs studios real money.

## 

step 6 — micro-interactions and ship (5 min)

before you deploy, add the micro-interactions that make a site feel alive — buttons that respond when you hover them, subtle parallax on the hero, a cursor that the page reacts to. small things, big difference.

paste this into claude code:

> "add micro-interactions across the site. buttons should scale up slightly on hover and press down on click, with a spring physics feel. add subtle parallax to the hero so the keyboard image shifts as the user moves their cursor. then review the entire build, fix any layout bugs on mobile, optimize all images, and prepare it for deploy."

claude will work through the codebase, add the interactions, catch mobile bugs, compress images, and hand you a build that's ready to go live.

once it's done, deploy in one line:

```text
bash
npx vercel --prod
```

site is live. total elapsed time: under 30 minutes if you don't get distracted.

under the hood — what claude actually added:

if you want to see what's making the buttons feel responsive, here's the pattern claude used:

```text
tsx
<motion.button
  whileHover={{ scale: 1.04 }}
  whileTap={{ scale: 0.97 }}
  transition={{ type: "spring", stiffness: 400, damping: 17 }}
>
  buy now
</motion.button>
```

three things going on:

— whileHover={{ scale: 1.04 }} — when the cursor hovers, the button grows by 4%. just enough to feel alive, not enough to feel cartoonish. — whileTap={{ scale: 0.97 }} — when clicked, it shrinks by 3%. mimics the physical "press" of a real button. this is what makes interfaces feel tactile. — type: "spring" — instead of a linear animation, the motion is governed by spring physics. stiffness: 400 makes it snappy; damping: 17 keeps it from oscillating. together they produce that "ios-feeling" bounce that's everywhere in modern product design.

four lines of code. the difference between a page that feels static and a page that feels designed.
