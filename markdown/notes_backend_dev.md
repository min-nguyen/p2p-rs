# Backend Web Development - Revision Notes

Backend web development involves building and maintaining the server-side of web applications, responsible for data processing, business logic, and database interactions.

---

## 1. Backend Languages & Frameworks

### Common Backend Languages:

- **Node.js (JavaScript/TypeScript)**:
  Asynchronous, event-driven runtime based on JavaScript.
  **Frameworks**: Express.js, Nest.js.

- **Python**:
  Known for simplicity and readability.
  **Frameworks**: Django (full-featured), Flask (minimalistic).

- **Ruby**:
  Emphasizes simplicity and productivity.
  **Framework**: Ruby on Rails.

- **Java**:
  Stable and widely used in enterprise applications.
  **Frameworks**: Spring Boot, Micronaut.

- **PHP**:
  Easy to learn and widely integrated with frontend development.
  **Frameworks**: Laravel, Symfony.

- **Go**:
  Compiled language known for its performance.
  **Frameworks**: Gin, Echo.

- **C#**:
  Primarily used in Microsoft ecosystems.
  **Framework**: ASP.NET Core.

### Choosing a Language:

- **JavaScript (Node.js)**: Great for full-stack development (MEAN/MERN stack).
- **Python (Django/Flask)**: Ideal for rapid development and flexibility.
- **Java/Go**: Suited for high-performance, scalable applications.

---

## 2. Backend Components

### 1. Routing & URL Management

Routing directs HTTP requests to specific handlers.

**Example (Express.js):**

```js
app.get('/users', (req, res) => {
  // Get users from database and send response
});
```
### 2. Handling HTTP Requests

Backend handles CRUD operations via HTTP methods (GET, POST, PUT, DELETE).


```python
@app.route('/users', methods=['GET'])
def get_users():
    return jsonify(users)
```

### 3. Middleware:

Middleware intercepts requests/responses, adding features like logging or authentication.

```js
app.use((req, res, next) => {
  console.log('Request URL:', req.url);
  next();
});
```

### 4. Authentication/Authorization:

Authentication verifies user identity (e.g., JWT, OAuth).
Authorization controls access based on user roles/permissions.
Tools: Passport.js (Node), Django Auth (Python), OAuth2, JWT.

### 5. Error Handling:

Handle server errors, exceptions, and validations gracefully.

```js
app.use((err, req, res, next) => {
  res.status(500).send({ message: 'Internal Server Error' });
});
```

## 3. Databases

Backend services interact heavily with databases to store and retrieve data.

### Types of Databases:

- **Relational Databases (SQL)**:
  - Examples: PostgreSQL, MySQL, SQLite.
  - Use SQL (Structured Query Language) to manage data in tables.
  - **ORM Tools**: Sequelize (Node.js), SQLAlchemy (Python).

- **NoSQL Databases**:
  - Examples: MongoDB, CouchDB.
  - Schemaless, stores JSON-like documents.
  - Ideal for unstructured or semi-structured data.
  - **ODM**: Mongoose (Node.js, MongoDB).

### Database Queries:

```sql
SELECT * FROM users WHERE age > 18;
```

### Connecting to Databases:

```js
         const mongoose = require('mongoose');
         mongoose.connect('mongodb://localhost:27017/mydb', { useNewUrlParser: true, useUnifiedTopology: true });
```

## 4. REST API & GraphQL

Backend applications often expose APIs for frontend consumption.

### REST API:

REST (Representational State Transfer) is an architectural style that uses HTTP methods to interact with resources.

- **HTTP Verbs**:
  - `GET`: Retrieve data.
  - `POST`: Create new data.
  - `PUT`: Update existing data.
  - `DELETE`: Remove data.

**Example of REST API CRUD operations:**

- `GET /users`: Retrieve all users.
- `POST /users`: Add a new user.
- `PUT /users/:id`: Update a specific user.
- `DELETE /users/:id`: Delete a specific user.


## 5. Deployment

Deploying a backend application involves hosting it on a server or platform, ensuring it is accessible to users and can handle traffic.

### 1. Servers & Cloud Platforms

You can host your backend using cloud platforms or dedicated servers.

- **Cloud Platforms**:
  - **Amazon Web Services (AWS)**: Provides a wide range of cloud computing services, such as EC2 for virtual servers and RDS for databases.
  - **Google Cloud Platform (GCP)**: Offers computing, storage, and machine learning services.
  - **Microsoft Azure**: Known for its enterprise solutions and integration with Windows services.

- **Platform as a Service (PaaS)**:
  - **Heroku** and **Vercel** are popular PaaS options that simplify deployment by handling server infrastructure.
  - With PaaS, you usually push your code to a Git repository, and the platform manages everything from scaling to server maintenance.

### 2. CI/CD Pipelines

Continuous Integration/Continuous Deployment (CI/CD) automates the process of testing, building, and deploying applications.

- **Popular CI/CD Tools**:
  - **GitHub Actions**: Integrates directly with GitHub repositories for running automated workflows.
  - **Travis CI**: A popular cloud-based CI service.
  - **Jenkins**: Open-source automation server widely used in CI/CD.

**Steps in a CI/CD pipeline**:
1. **Commit code to a version control system** (e.g., Git).
2. **Run automated tests** to ensure new changes don’t break existing functionality.
3. **Build the application** if the tests pass.
4. **Deploy to production**.