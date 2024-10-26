# Covers-API
## Status of Development
**Work in Progress**
## Description
Covers API is a backend service that offers an API to download/manage covers for products.  
It will fetch on configured providers the image associated with a barcode.
## Possible use-case
Anything that needs covers.  
For example an ecommerce.
## Objective
Priority is to make it work with specific backends. Modularity for other backends will come after.
## Installation
### Requirements
- pass binary in PATH
- mysql database write access
- task tracker API  
  Supported: [Tasks-tracker](https://github.com/Cyrix126/tasks-tracker)  
- product API  
  Supported: [Dolibarr](https://github.com/Dolibarr/dolibarr)
#### Optional requirements
- caching-proxy.  
  Supported:  [Mnemosyne](https://github.com/Cyrix126/Mnemosyne)
## Licence
covers-api is GPL 3. See [Licence](LICENCE.md)
