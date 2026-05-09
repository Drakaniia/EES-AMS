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
