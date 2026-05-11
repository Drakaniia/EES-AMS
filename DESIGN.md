---
version: 'neuform-top-creators-featured'
name: 'Initialize Account'
description: 'Initialize Account Login Section is designed for authenticating users through a focused access flow. Key features include reusable layout structure, responsive behavior, and production-ready styling. Built with custom CSS, it is suitable for authentication screens in web products.'
colors:
  primary: '#EA580C'
  secondary: '#FDFBF9'
  accent: '#C2410C'
  background: '#FDFBF9'
  surface: '#F0EBE1'
  text-primary: '#111827'
  text-secondary: '#4B5563'
  border: '#292524'
typography:
  display-lg:
    fontFamily: 'Inter'
    fontSize: '64px'
    fontWeight: 500
    lineHeight: '1.04'
    letterSpacing: '0'
  body-md:
    fontFamily: 'Inter'
    fontSize: '16px'
    fontWeight: 400
    lineHeight: '1.6'
  label-md:
    fontFamily: 'JetBrains Mono'
    fontSize: '12px'
    fontWeight: 600
    lineHeight: '1.2'
spacing:
  base: '8px'
  gap: '16px'
  card-padding: '24px'
  section-padding: '80px'
rounded:
  card: '16px'
  control: '8px'
  pill: '9999px'
components:
  card:
    background: 'Use the surface token with subtle borders and HTML-matched shadow depth'
    radius: 'Match the declared card radius token'
  button:
    background: 'Use primary or accent colors for the main action'
    radius: 'Use the control or pill radius based on the source HTML'
---

## Composition

Use the attached HTML reference as the source of truth. Preserve the visible hierarchy, first-screen composition, section rhythm, density, and interaction tone before adapting copy or content.
Key visible headings include: Establish your Horizon workspace; Distribute intelligent corporate cards.

## Colors

Anchor the palette in primary #EA580C, secondary #FDFBF9, accent #C2410C, background #FDFBF9, surface #F0EBE1, text-primary #111827. Keep background, surface, text, and border roles distinct so generated layouts retain the same contrast pattern as the source.

## Typography

Use Inter for display moments and Inter for body copy unless the HTML clearly demands a compatible fallback. Labels and technical metadata should use JetBrains Mono or an equivalent mono face.

## Layout

Keep spacing deliberate and stable. Favor the same grid direction, max-width behavior, card density, and responsive stacking seen in the HTML. Do not replace distinctive source structures with generic SaaS sections.

## Components

Authentication and CTA controls should preserve the source button hierarchy, input density, and focused conversion path.

## Motion

Preserve existing motion cues such as masked reveals, staggered entrance, hover lift, scroll-triggered transitions, and ambient movement. Keep easing smooth and restrained.

## WebGL & Effects

If the source includes canvas, WebGL, Three.js, gradients, particles, or atmospheric effects, rebuild them as supporting layers behind the content. Keep effects performant, responsive, and secondary to the interface.

## Guardrails

- Do not flatten the source into a generic card grid.
- Do not swap the color mode unless the source clearly supports it.
- Preserve the first viewport signal, focal object, and visual density.
- Keep buttons, cards, and badges aligned to the same radius and border language.

- Orange Clean Paper SaaS Skill
  Scope:
- Apply this as a full design-system direction across page shell, forms, product mockups, cards, CTAs, illustration zones, and motion.
- Use it when the interface should feel like a refined SaaS onboarding or product experience built on warm paper tones rather than cold white dashboards.
- This is not generic startup UI and not a purely technical paper system. It should feel welcoming, polished, and product-led while staying premium.
  Visual target:
- Build the interface with warm off-white, cream, parchment, and pale stone surfaces instead of stark white.
- Use orange as the primary signal and action color for steps, buttons, active states, icons, focused inputs, and small emphasis details.
- Keep the overall system clean and modern, with generous radius, soft shadows, and carefully layered surfaces that feel tactile but not skeuomorphic.
- Pair a functional form or onboarding area with a polished visual product panel, such as illustrated cards, floating stats, or a dimensional app object.
- Let the mood feel calm and premium, with orange used as a warm energetic accent rather than a loud marketing blast.
  Implementation guidance:
- Prefer a large rounded master container with subtle gradient-border treatment and a warm background shell around the main UI.
- Build forms with high-quality light inputs: paper-toned fill, delicate borders, subtle focus rings, and soft hover states tied to the orange accent.
- Use orange in a disciplined way for step markers, primary CTAs, highlight icons, link text, micro-badges, and small progress indicators.
- Create a companion product-illustration region using warm gradients, floating cards, soft glassy white UI panels, or rendered product objects to communicate the platform visually.
- Use clean sans-serif typography, light-to-regular weight body copy, and restrained hierarchy so the experience feels elegant and approachable.
- Motion should stay polished and product-grade: masked text reveals, gentle floating elements, smooth input focus transitions, and calm ambient movement in the illustration zone.
  Recommended patterns:
- Split onboarding card with a form panel on one side and a visual product demonstration panel on the other.
- Warm paper surfaces layered inside a slightly brighter or softer radial page background.
- Rounded white or cream cards with light gradient borders and soft orange-tinted shadows.
- Floating UI chips, balance cards, card objects, or product visuals that add depth without clutter.
- Orange primary button paired with quiet secondary actions and neutral dividers to preserve the clean SaaS tone.
  Tuning knobs:
- Paper warmth: shift between cooler cream and richer parchment depending on the brand feel.
- Orange energy: keep the accent vivid enough to feel active, but restrained enough that the interface remains calm.
- Radius and softness: use generous rounding and shadow lift, but avoid turning the system into bubbly consumer UI.
- Visual richness: add or reduce floating product elements depending on how illustration-heavy the page should be.
- Contrast: preserve clear readability between warm backgrounds, white cards, gray copy, and orange actions.
  Avoid:
- Cold blue-gray SaaS UI that ignores the warm paper character.
- Flat white forms with no softness, product depth, or premium surface treatment.
- Oversaturating the whole page with orange instead of using it as a signal color.
- Heavy vintage paper distressing that makes the SaaS product feel old-fashioned.
- Overcomplicated illustrations or noisy 3D objects that distract from the clean onboarding experience.
