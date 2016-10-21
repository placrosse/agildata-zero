---
layout: default
title: AgilData Zero - Configuration
active: configuration
---

# Configuration

In this first phase of AgilData Zero, database and gateway configuration is in the form of a client-side XML config. Later iterations could likely
replace this with integrations with key management systems, encrypted in-database metadata configuration, and/or out-of-band UI configuration tools.

The default XML config comes shipped with the release tar: `./zero-config.xml`

Configuration is straight forward: 

```xml
<zero-config>
    
    <client>
    <client>
</zero-config>
```

Edit the XML

```xml
<this>
  <is>
     <awesome>
```
