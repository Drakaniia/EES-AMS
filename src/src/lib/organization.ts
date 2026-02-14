import { User } from "firebase/auth";

export interface Organization {
  domain: string;
  name: string;
  type: 'educational' | 'government' | 'corporate' | 'other';
  logo?: string;
}

export interface UserProfile {
  uid: string;
  email: string;
  displayName: string;
  photoURL?: string;
  organization?: Organization;
  schoolName?: string;
  position?: string;
  department?: string;
  employeeId?: string;
  createdAt: Date;
  lastLogin: Date;
}

export const DEPED_ORGANIZATION: Organization = {
  domain: "deped.gov.ph",
  name: "Department of Education",
  type: "government",
  logo: ""
};

export const detectOrganization = (email: string): Organization | undefined => {
  if (!email || !email.includes("@")) return undefined;

  const domain = email.split("@")[1]?.toLowerCase();
  
  if (!domain) return undefined;

  // Common educational and government domains in Philippines
  const organizationMap: Record<string, Organization> = {
    "deped.gov.ph": DEPED_ORGANIZATION,
    "ched.gov.ph": {
      domain: "ched.gov.ph",
      name: "Commission on Higher Education",
      type: "government"
    },
    "dost.gov.ph": {
      domain: "dost.gov.ph", 
      name: "Department of Science and Technology",
      type: "government"
    },
    "up.edu.ph": {
      domain: "up.edu.ph",
      name: "University of the Philippines",
      type: "educational"
    },
    "dlsu.edu.ph": {
      domain: "dlsu.edu.ph",
      name: "De La Salle University",
      type: "educational"
    },
    "ateneo.edu": {
      domain: "ateneo.edu",
      name: "Ateneo de Manila University",
      type: "educational"
    },
    "ust.edu.ph": {
      domain: "ust.edu.ph",
      name: "University of Santo Tomas",
      type: "educational"
    }
  };

  // Check for exact domain match
  if (organizationMap[domain]) {
    return organizationMap[domain];
  }

  // Check for subdomains
  for (const [orgDomain, org] of Object.entries(organizationMap)) {
    if (domain === orgDomain || domain.endsWith(`.${orgDomain}`)) {
      return org;
    }
  }

  // Generic domain detection
  if (domain.endsWith(".gov.ph")) {
    return {
      domain,
      name: domain.split(".")[0]?.toUpperCase() || "Government Agency",
      type: "government"
    };
  }

  if (domain.endsWith(".edu.ph") || domain.endsWith(".edu")) {
    return {
      domain,
      name: domain.split(".")[0]?.toUpperCase() || "Educational Institution",
      type: "educational"
    };
  }

  return undefined;
};

export const createProfileFromUser = (user: User): UserProfile => {
  const organization = user.email ? detectOrganization(user.email) : undefined;
  
  return {
    uid: user.uid,
    email: user.email || "",
    displayName: user.displayName || user.email?.split("@")[0] || "",
    photoURL: user.photoURL || "",
    organization,
    schoolName: organization?.name || "",
    position: "",
    department: "",
    employeeId: "",
    createdAt: new Date(),
    lastLogin: new Date()
  };
};