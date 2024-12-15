#ifndef CURSEOFRUST_H
#define CURSEOFRUST_H

#include <cstdint>
#include <stdint.h>

typedef struct {
  void *first;
  void *second;
} CORFunctionReturn;
typedef struct {
  int32_t x;
  int32_t y;
} CORPosition;
typedef struct {
  CORPosition cursor;
  uint16_t xskip;
  uint16_t xlen;
} CORInterface;

typedef void *CORBasicOptsRef;
typedef void *CORMultiplayerOptsRef;
typedef void *CORStateRef;

CORFunctionReturn CORParseOptions(char *_Nonnull optStringPtr);
void CORReleaseErrorString(char *_Nonnull errorStringPtr);
CORFunctionReturn CORMakeState(CORBasicOptsRef _Nonnull basicOptsPtr);
CORInterface CORMakeInterface(CORStateRef _Nonnull statePtr);
uint64_t CORGetSeed(CORStateRef _Nonnull statePtr);
uint32_t CORGetGridHeight(CORStateRef _Nonnull statePtr);
uint32_t CORGetGridWidth(CORStateRef _Nonnull statePtr);
void CORKingsMove(CORStateRef _Nonnull statePtr);
void CORSimulate(CORStateRef _Nonnull statePtr);

#endif