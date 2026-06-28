# SRAM22 Website Specification

The SRAM22 website will be hosted at `https://sram22.com`.

It should be a site written using the latest version of the [Astro](https://astro.build/) framework.
Follow idiomatic Astro conventions.

## Landing Page

The root/landing page should have a nice icon and large, clearly visible links to the
SRAM22 [GitHub repo](https://github.com/rahulk29/sram22) and to the documentation pages.

## Documentation Pages

These should be hosted under the `/docs` URL.

Note that SRAM22 SRAMs are meant to be rotated by 90 degrees, so there are 4 legal orientations of the SRAMs (R90, R270, and the corresponding mirrored variants).
Make sure to clarify this in the appropriate places in the docs pages.

The docs pages should include the following subpages.

* Quickstart: minimal set of steps to install SRAM22 and generate an SRAM
* Tutorial
    * With OpenROAD: how to generate an SRAM22 SRAM and integrate it into the OpenROAD flow
    * With Cadence tools: how to generate an SRAM22 SRAM and integrate it into a flow using Genus/Innovus
* Interface: have subsections for the pin list, pin positions, and timing diagrams. Pin list: a description of the SRAM interface. Include a table listing the pins of the SRAM, showing the pin name, direction, width, and a concise explanation of their purpose.
  Pin positions: a widget that allows users to see the physical location of each pin on an overlay of the SRAM layout. The widget should be interactive and allow panning/zooming/etc.
  Timing diagrams: show (pictorially) the relationships between the pins (e.g. upon clk, if ce=1 and we=0, dout changes to read out data at addr) and relevant properties such as setup/hold time, clk-q time, etc.
* Internals
    * Waveforms: shows key internal waveforms for the SRAM (e.g. showing replica vs main bitline)
    * Layout: allows users to browse the SRAM layout in the browser
    * Algorithms: a detailed explanation (including diagrams) of the algorithms used by SRAM22. Organize into sections. For example, have a section on decoder sizing.

## Aesthetics

Use the following color palette:

```
[{"name":"Yale Blue","hex":"274060","rgb":[39,64,96],"cmyk":[59,33,0,62],"hsb":[214,59,38],"hsl":[214,42,26],"lab":[27,1,-22]},{"name":"Baltic Blue","hex":"335c81","rgb":[51,92,129],"cmyk":[60,29,0,49],"hsb":[208,60,51],"hsl":[208,43,35],"lab":[38,-2,-25]},{"name":"Cool Sky","hex":"65afff","rgb":[101,175,255],"cmyk":[60,31,0,0],"hsb":[211,60,100],"hsl":[211,100,70],"lab":[70,0,-47]},{"name":"Space Indigo","hex":"1b2845","rgb":[27,40,69],"cmyk":[61,42,0,73],"hsb":[221,61,27],"hsl":[221,44,19],"lab":[16,5,-20]},{"name":"Cloudy Sky","hex":"5899e2","rgb":[88,153,226],"cmyk":[61,32,0,11],"hsb":[212,61,89],"hsl":[212,70,62],"lab":[62,1,-43]}]
```

Use Fontsource to install fonts into the Astro project.
Use [Geist](https://fontsource.org/fonts/geist) for the main font for headings and prose.
Use [JetBrains Mono](https://fontsource.org/fonts/jetbrains-mono) for code and CLI commands.

Generate a nice, modern logo for SRAM22.

## Consistency

Design and implement a mechanism for checking that documentation is up-to-date (e.g. as part of a CI job).
