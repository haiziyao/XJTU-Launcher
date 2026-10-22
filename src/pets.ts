// The six pets are homophones of 西安交大同学: 嘻嘻 安安 娇娇 彤彤 哒哒 雪雪.
export type PetId = 'xixi' | 'anan' | 'jiaojiao' | 'tongtong' | 'dada' | 'xuexue';

export interface PetOption {
  id: PetId;
  name: string;
}

export const PETS: PetOption[] = [
  { id: 'xixi', name: '嘻嘻' },
  { id: 'anan', name: '安安' },
  { id: 'jiaojiao', name: '娇娇' },
  { id: 'tongtong', name: '彤彤' },
  { id: 'dada', name: '哒哒' },
  { id: 'xuexue', name: '雪雪' },
];

export const DEFAULT_PET: PetId = 'xixi';

export function normalizePet(value: unknown): PetId {
  return PETS.some((pet) => pet.id === value) ? (value as PetId) : DEFAULT_PET;
}
