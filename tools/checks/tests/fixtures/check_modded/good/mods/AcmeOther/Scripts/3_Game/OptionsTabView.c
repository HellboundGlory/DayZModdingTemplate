// Why: Default has to reach every row of a tab built from the framework's options views.
// Removing this leaves the tab's rows on their saved values.
// Layer: 3_Game, where the view is declared.
// Other mods patching this class: none in the reference mods.

modded class OptionsTabView
{
    bool AcmeCoreShowDefault()
    {
        return true;
    }
}
