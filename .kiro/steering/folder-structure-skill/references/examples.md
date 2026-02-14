# Folder Structure Examples

## React/TypeScript Examples

### Feature-Based Structure Example
```
src/
├── features/
│   ├── authentication/
│   │   ├── components/
│   │   │   ├── LoginForm/
│   │   │   │   ├── LoginForm.tsx
│   │   │   │   ├── LoginForm.styles.ts
│   │   │   │   └── index.ts
│   │   │   ├── SignupForm/
│   │   │   └── PasswordReset/
│   │   ├── hooks/
│   │   │   ├── useAuth.ts
│   │   │   └── useAuthState.ts
│   │   ├── services/
│   │   │   ├── authService.ts
│   │   │   └── authApi.ts
│   │   ├── types/
│   │   │   ├── auth.types.ts
│   │   │   └── user.types.ts
│   │   └── index.ts
│   ├── dashboard/
│   │   ├── components/
│   │   │   ├── DashboardLayout/
│   │   │   ├── StatisticsCard/
│   │   │   └── RecentActivity/
│   │   ├── hooks/
│   │   │   ├── useDashboardData.ts
│   │   │   └── useRealTimeUpdates.ts
│   │   ├── services/
│   │   │   └── dashboardService.ts
│   │   └── index.ts
│   └── userProfile/
├── shared/
│   ├── components/
│   │   ├── Button/
│   │   ├── Modal/
│   │   ├── Input/
│   │   └── LoadingSpinner/
│   ├── hooks/
│   │   ├── useApi.ts
│   │   ├── useLocalStorage.ts
│   │   └── useDebounce.ts
│   ├── utils/
│   │   ├── dateUtils.ts
│   │   ├── validationUtils.ts
│   │   └── formatUtils.ts
│   ├── types/
│   │   ├── common.types.ts
│   │   └── api.types.ts
│   └── constants/
│       ├── apiEndpoints.ts
│       └── appConstants.ts
├── assets/
│   ├── images/
│   ├── icons/
│   └── fonts/
├── styles/
│   ├── globals.css
│   ├── variables.css
│   └── themes/
│       ├── light.css
│       └── dark.css
└── App.tsx
```

## Node.js/Express Examples

### Layered Structure Example
```
src/
├── controllers/
│   ├── authController.js
│   ├── userController.js
│   └── productController.js
├── services/
│   ├── authService.js
│   ├── userService.js
│   └── productService.js
├── models/
│   ├── User.js
│   ├── Product.js
│   └── Order.js
├── routes/
│   ├── authRoutes.js
│   ├── userRoutes.js
│   └── productRoutes.js
├── middleware/
│   ├── auth.js
│   ├── validation.js
│   └── errorHandler.js
├── utils/
│   ├── database.js
│   ├── logger.js
│   └── helpers.js
├── config/
│   ├── database.js
│   ├── environment.js
│   └── security.js
└── app.js
```

## Clean Architecture Example

### Domain Layer
```
src/domain/
├── entities/
│   ├── User.ts
│   ├── Product.ts
│   └── Order.ts
├── services/
│   ├── UserDomainService.ts
│   ├── OrderDomainService.ts
│   └── PaymentDomainService.ts
├── repositories/
│   ├── IUserRepository.ts
│   ├── IProductRepository.ts
│   └── IOrderRepository.ts
├── events/
│   ├── UserRegistered.ts
│   ├── OrderPlaced.ts
│   └── PaymentProcessed.ts
└── errors/
    ├── UserNotFoundError.ts
    ├── InsufficientStockError.ts
    └── PaymentFailedError.ts
```

### Infrastructure Layer
```
src/infrastructure/
├── database/
│   ├── repositories/
│   │   ├── UserRepository.ts
│   │   ├── ProductRepository.ts
│   │   └── OrderRepository.ts
│   ├── migrations/
│   └── seeds/
├── external/
│   ├── emailService.ts
│   ├── paymentGateway.ts
│   └── fileStorage.ts
├── config/
│   ├── database.ts
│   ├── email.ts
│   └── redis.ts
└── middleware/
    ├── authMiddleware.ts
    ├── loggingMiddleware.ts
    └── errorMiddleware.ts
```

### Application Layer
```
src/application/
├── commands/
│   ├── RegisterUserCommand.ts
│   ├── CreateOrderCommand.ts
│   └── ProcessPaymentCommand.ts
├── queries/
│   ├── GetUserByIdQuery.ts
│   ├── GetProductListQuery.ts
│   └── GetOrderHistoryQuery.ts
├── handlers/
│   ├── commandHandlers/
│   └── queryHandlers/
├── useCases/
│   ├── user/
│   │   ├── RegisterUserUseCase.ts
│   │   └── AuthenticateUserUseCase.ts
│   ├── product/
│   │   ├── GetProductsUseCase.ts
│   │   └── UpdateStockUseCase.ts
│   └── order/
│       ├── CreateOrderUseCase.ts
│       └── CancelOrderUseCase.ts
└── interfaces/
    ├── IHttpController.ts
    └── IEventPublisher.ts
```

## Import Path Examples

### TypeScript Configuration
```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"],
      "@/domain/*": ["src/domain/*"],
      "@/infrastructure/*": ["src/infrastructure/*"],
      "@/application/*": ["src/application/*"],
      "@/presentation/*": ["src/presentation/*"],
      "@/shared/*": ["src/shared/*"],
      "@/features/*": ["src/features/*"]
    }
  }
}
```

### Example Imports
```typescript
import { User } from '@/domain/entities/User';
import { UserRepository } from '@/infrastructure/database/repositories/UserRepository';
import { RegisterUserUseCase } from '@/application/useCases/user/RegisterUserUseCase';

// vs

import { Button } from '@/shared/components/Button';
import { LoginForm } from '@/features/authentication/components/LoginForm';
import { useAuth } from '@/features/authentication/hooks/useAuth';
```

## Migration Examples

### From Type-Based to Feature-Based

**Before (Type-Based):**
```
src/
├── components/
│   ├── LoginForm.tsx
│   ├── SignupForm.tsx
│   ├── ProductCard.tsx
│   └── UserProfile.tsx
├── hooks/
│   ├── useAuth.ts
│   └── useProducts.ts
└── utils/
    ├── authUtils.ts
    └── productUtils.ts
```

**After (Feature-Based):**
```
src/
├── features/
│   ├── authentication/
│   │   ├── components/
│   │   │   ├── LoginForm.tsx
│   │   │   └── SignupForm.tsx
│   │   ├── hooks/
│   │   │   └── useAuth.ts
│   │   ├── utils/
│   │   │   └── authUtils.ts
│   │   └── index.ts
│   └── products/
│       ├── components/
│       │   └── ProductCard.tsx
│       ├── hooks/
│       │   └── useProducts.ts
│       └── utils/
│           └── productUtils.ts
└── shared/
    ├── components/
    └── utils/
```

## Team Collaboration Examples

### Ownership Matrix
| Domain | Owner | Contributors | Contact |
|--------|-------|--------------|---------|
| authentication | Team A | - | @team-auth |
| products | Team B | Team A (auth) | @team-products |
| orders | Team C | Team B, Team A | @team-orders |

### Integration Points
```
src/
├── features/
│   ├── authentication/          # Team A
│   ├── products/               # Team B
│   └── orders/                 # Team C
├── shared/
│   ├── interfaces/             # Integration contracts
│   ├── events/                 # Cross-domain events
│   └── types/                  # Shared type definitions
└── integration/
    ├── auth-products/          # Auth <-> Products integration
    └── auth-orders/            # Auth <-> Orders integration
```