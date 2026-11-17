// Why: a car keeps its owner across restarts, and other mods read it.
// Removing this loses the owner.
// Layer: 4_World, because CarScript is defined here.
// Other mods patch CarScript too: see the conflict register.

modded class CarScript
{
    protected string m_AcmeCoreOwner;
}
