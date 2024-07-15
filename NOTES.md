## Tables
A table is created if it does not exist:  

covers  
id,has_image,last_try,provider  
int(11),bool,date,varchar(255)

The table for product must include the following columns:  
rowid,ref
## Configuration
All external API connection are set in the configuration file.
passwords must be a path to a pass file.
## Status of Task
Covers-api will update a task manager with a running task.  
When a response construction from cover API does not need to connect to other API (for example if only a sql request or io access is needed), no task tracking is created and the response is returned once the task is completed. 
Otherwise, covers-api will create a task tracking. It will execute it while listening if it has been canceled. If it was, it will abort the job. If the task is not canceled and finish, the status will be updated as "done".
## backend APIs
Covers-API need the following API to work:

- **Product API**:  
to check if id of product exist  
to get the barcode of a product from his id  
to get the title of a product from his id  

- **Task Tracker API**:  
to create a task tracker when a request need to perform a long operation.

- **Cache API**  
to update the cache when a resource is modified.

These API could have different endpoints. The administrator will indicate which type of API for each covers-API will need to use.  
In first versions of cover-API, only the backend API included can be chosen. In futures versions, requests and post-processing can be added in the configuration file to add more support.
## Images
Images are converted to lossless webp.
reason to not use avif is that lossless compression is less efficient than webp.
## Resources
https://restfulapi.net
https://siipo.la/blog/whats-the-best-lossless-image-format-comparing-png-webp-avif-and-jpeg-xl 
