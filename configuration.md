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
    
    <!-- proxy configurations -->
    <client>
        <property name="host" value="127.0.0.1" />
        <property name="port" value="3307" />
    </client>
    
    <!-- remote mysql configurations -->
    <connection>
        <property name="host" value="127.0.0.1"/>
        <property name="user" value="myuser"/>
        <property name="password" value="mypassword"/>
    </connection>
    
    <!-- 
        parsing settings 
        mode allows strict|permissive
    -->
    <parsing>
        <property name="mode" value="strict"/>
    </parsing>
    
    <!-- schema configurations, one for each unique schema name -->
    <schema name="myschema>
    
        <!-- table configurations, one for each unique table name -->
        <table name="table1>
        
            <!-- column configurations, one for each unique column name
                type: Any supported mysql data type, using SQL syntax
                encryption: NONE|AES|AES_GCM
                
                AES and AES_GCM require:
                key: Hex string of 32 bytes
                
                AES requires:
                iv: Initialization vector, Hex string of 12 bytes
                
            -->
            <column name="id" type="INTEGER" encryption="none" />
            <column name="a" type="VARCHAR(16)" encryption="AES" iv="03F72E7479F3E34752E4DD91" key="44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985"/>
            <column name="b" type="FLOAT" encryption="AES_GCM" key=".." />
           
        </table>
        
        <table name="table2">
            <!-- etc -->
        </table>
    </schema>
    
    <schema name="myotherschema">
        <!-- tables... -->
    </schema>
    
</zero-config>
```

When executing the `agildata-zero` executable, the default config location can be overridden with the `--config` option.

Zero also supports the use of config fragments to extend or override components of your default config. Any such config fragments must be placed in the `/etc/config.d/` directory.
This can be advantageous when changing configurations often during development, when integrating with Docker, or compartmentalizing verbose schema configurations.
 
One example of a config fragment can be to override a certain element, such as connection:
 
```xml
<!-- overrides connection properties in the default config -->
<zero-config>
     <connection>
         <property name="host" value="176.120.90.168" />
         <property name="user" value="somecustomuser" />
         <property name="password" value="s0m3cust0mP@$$w0rd" />
     </connection>
</zero-config>
```
 
Another can be to extend other configs, such as adding new schema configuration:
 
```xml
<!-- extends a schema declaration  -->
<zero-config>
     <schema name="newschema">
         <table name="newtable">
             <column name="id" type="INTEGER" encryption="none"/>
             <column name="a" type="VARCHAR(50)" encryption="AES" iv="..." key="..."/>
             <column name="b" type="VARCHAR(50)" encryption="AES_GCM" key="..."/>
         </table>
     </schema>
</zero-config>
```
