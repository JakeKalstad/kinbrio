# Kinbrio

Business management platform to manage small business. Integration with LLama2 and Akaunting.

`sudo apt update`

`sudo apt upgrade`

`sudo apt install build-essential`

`curl https://sh.rustup.rs -sSf | sh`

installs rust/cargo and voila you're done (https://doc.rust-lang.org/cargo/getting-started/installation.html)

`sudo apt-get install postgresql-14`

install postgresql

    `sudo su - postgres`

    `createuser projectmanager --pwprompt`
     set password: `projectmanager123`

    `createdb projectmanager`
	
	use the postgres cmd line client
	`psql`
	
	`GRANT ALL PRIVILEGES ON DATABASE projectmanager TO projectmanager;`
You can change the user and password in the createuser call and even the database in the createdb call, but the .env file has to reflect them in the DATABASE_URL variable so it points to what you have setup locally.


# RUN IT

to run the app it's `cargo run` same for the kinbot/matrix-bot project.

`sh start-gpt.sh` will start a server for the AI assistant dashboard widget we frame in.
`sh startt-matrix-bot.sh` 
# SPECS 

## Project management MVP
Create an organization + user

Add Users to your organization + assign them roles

Add a project to your organization 

Add tasks to a project

Add a milestone to this project, time spent / particular or % of tasks completed / dates hit / etc 

View list of tasks

View calendar view of tasks for project

View calendar view of tasks for organization

Add a board to add visualizations for a project or for your organziation 
    Used to select and display data in a custom way for the user 

Everything can have a file or note attached to it and a way for the users to view them

## Relationship manager MVP

Create an external organization and contact(s)

### Core Data:

#### ENTITY (Business, Vendor, Client, Customer) 
	-> Contact(s) Information
	-> sensitive PII, jobs and processing for accessibility/exporting/integrating/scrambling

#### Service Item (Item, service rendered, contactual obligation)
	-> Name
	-> Details
	-> 

#### Transaction (Point of sale, service completed, contract closed) 
    these should be templatable, people will reuse more than create from scratch 
    so the label/details/etc(this will surely grow), can be preconfigured 
    (but overridable at creation))
	-> Entity involved + point person(s) contact
	-> Label
	-> Details
	-> final dollar amount if applicable
	-> date expected vs date actual (service completion / contract closing will vary)
	
#### APPOINTMENT (Future Transaction)
	-> Transaction
	-> Date + Duration
	-> Tentative dollar amount if applicable

### Core features

Matrix server integration - client integrations for workflow !automations

Contact management (contacts/leads)

Service/Item organization

appointment management/project calenders

Reporting/projections/deriving from data, potential vs actual, timelines etc

### Relation management use case 
Fencing company: Suppliers + Customers, appointments for upcoming jobs and deliveries

T-Shirt company: Suppliers + Customers -> emailing promotions/ads, importing potential customers from lead generation campaigns

Marketing broker: Lead generators and lead consumers, acquiring and forwarding lists of leads on to 3rd partys via API/Export/Etc

Saas Service Provider: Customer and account integration via API to manage user accounts and schedule appointments for onboardings/support calls/etc

Contractors: Customers, sub contractors, clients, potential clients + appointments for scheduling work and client deadlines.

Software consulting business: Clients, potential clients, internal/external employees + appointment management, contracts potential/in flight/closed  


https://santurcesoftware.com
